//! Entirely local simulation: no account, credential, URL opening, or HTTP calls.

use crate::auto_repos::auto_select;
use crate::models::*;
use crate::workflow::{validate_inputs, validate_non_secret_inputs};
use chrono::Utc;
use std::collections::BTreeMap;
use std::time::{Duration, Instant};

const FIXTURE: &str = include_str!("../../fixtures/demo.json");
const DEMO_MESSAGE: &str = "Demo data only. No GitHub connection or real workflow starts.";

#[derive(Clone)]
struct RecordedRequest {
    request: DispatchRequest,
    outcome: DispatchOutcome,
}

struct SimulatedRun {
    started: Instant,
    initial_status: RunStatus,
}

pub struct DemoStore {
    snapshot: Snapshot,
    scenario: DemoScenario,
    next_run_id: u64,
    requests: RecentOutcomes<(String, String), RecordedRequest>,
    /// Locked targets with their claim time; only `release_uncertain_start` or a reset clears them.
    uncertain_targets: BTreeMap<DispatchTarget, String>,
    simulated_runs: BTreeMap<u64, SimulatedRun>,
}

impl Default for DemoStore {
    fn default() -> Self {
        Self::new()
    }
}

impl DemoStore {
    pub fn new() -> Self {
        let mut snapshot: Snapshot = serde_json::from_str(FIXTURE)
            .expect("bundled demo fixture must match the checked TypeScript contract");
        // The same automatic rule as a live catalog listing; the demo hides and filters nothing.
        auto_select(&mut snapshot.repos, &snapshot.pins, &[], &[]);
        let started = Instant::now();
        let simulated_runs = snapshot
            .runs
            .iter()
            .filter(|run| matches!(run.status, RunStatus::Queued | RunStatus::InProgress))
            .map(|run| {
                (
                    run.id,
                    SimulatedRun {
                        started,
                        initial_status: run.status,
                    },
                )
            })
            .collect();
        let next_run_id = snapshot.runs.iter().map(|run| run.id).max().unwrap_or(0) + 1;
        Self {
            snapshot,
            scenario: DemoScenario::Normal,
            next_run_id,
            requests: RecentOutcomes::new(MAX_RECORDED_OUTCOMES),
            uncertain_targets: BTreeMap::new(),
            simulated_runs,
        }
    }

    pub fn snapshot(&self) -> Snapshot {
        let mut snapshot = self.snapshot.clone();
        snapshot.uncertain_starts = self
            .uncertain_targets
            .iter()
            .map(|(target, claimed_at)| UncertainStart {
                account_id: target.account_id.clone(),
                repo_id: target.repo_id.clone(),
                workflow_id: target.workflow_id,
                git_ref: target.git_ref.clone(),
                claimed_at: Some(claimed_at.clone()),
            })
            .collect();
        snapshot
    }

    /// Mirrors the live command: releases exactly the reviewed claim and never dispatches.
    pub fn release_uncertain_start(&mut self, start: UncertainStart) -> Result<Snapshot, String> {
        let target = start.target();
        match self.uncertain_targets.get(&target) {
            Some(claimed_at) if start.claimed_at.as_ref() != Some(claimed_at) => {
                return Err(crate::i18n::tr(
                    "Für diesen Workflow auf diesem Branch oder Tag gibt es eine neuere Startsperre. Prüfe die Runs erneut.",
                    "This workflow has a newer start lock on this branch or tag. Check the runs again.",
                )
                .into())
            }
            Some(_) => {
                self.uncertain_targets.remove(&target);
            }
            None => {}
        }
        Ok(self.snapshot())
    }

    pub fn refresh(&mut self) -> Snapshot {
        self.refresh_at(Instant::now())
    }

    fn refresh_at(&mut self, now: Instant) -> Snapshot {
        if self.snapshot.connectivity == Connectivity::Online {
            let timestamp = Utc::now().to_rfc3339();
            for run in &mut self.snapshot.runs {
                if let Some(simulation) = self.simulated_runs.get(&run.id) {
                    let elapsed = now.saturating_duration_since(simulation.started);
                    let complete_after = match simulation.initial_status {
                        RunStatus::InProgress => Duration::from_secs(1),
                        _ => Duration::from_secs(2),
                    };
                    let new_status = if elapsed >= complete_after {
                        RunStatus::Completed
                    } else if elapsed >= Duration::from_secs(1) {
                        RunStatus::InProgress
                    } else {
                        simulation.initial_status
                    };
                    if new_status != run.status {
                        run.status = new_status;
                        run.updated_at = timestamp.clone();
                        if new_status == RunStatus::Completed {
                            run.conclusion = Some(Conclusion::Success);
                        }
                    }
                }
            }
            let completed: Vec<_> = self
                .snapshot
                .runs
                .iter()
                .filter(|run| run.status == RunStatus::Completed)
                .map(|run| run.id)
                .collect();
            for id in completed {
                self.simulated_runs.remove(&id);
            }
            self.snapshot.updated_at = timestamp;
            self.snapshot.stale = false;
        }
        self.snapshot()
    }

    pub fn save_pin(&mut self, mut pin: Pin) -> Result<Snapshot, String> {
        validate_pin_fields(&pin)?;
        check_pin_room(&self.snapshot.pins, &pin.id)?;
        validate_non_secret_inputs(&pin.inputs).map_err(|error| error.to_string())?;
        let request = DispatchRequest {
            account_id: pin.account_id.clone(),
            repo_id: pin.repo_id.clone(),
            workflow_id: pin.workflow_id,
            git_ref: pin.git_ref.clone(),
            inputs: pin.inputs.clone(),
            request_id: "pin-validation".into(),
        };
        pin.inputs = self.validate_request(&request)?;
        // Defaults can introduce secret fields too, so validate the final persisted inputs.
        validate_non_secret_inputs(&pin.inputs).map_err(|error| error.to_string())?;
        if let Some(existing) = self.snapshot.pins.iter_mut().find(|old| old.id == pin.id) {
            if existing.account_id != pin.account_id {
                return Err("A pin ID cannot be reassigned to another account.".into());
            }
            *existing = pin;
        } else {
            self.snapshot.pins.push(pin);
        }
        Ok(self.snapshot())
    }

    pub fn delete_pin(&mut self, id: String) -> Result<Snapshot, String> {
        let index = self
            .snapshot
            .pins
            .iter()
            .position(|pin| pin.id == id)
            .ok_or_else(|| "The pin no longer exists.".to_string())?;
        self.snapshot.pins.remove(index);
        Ok(self.snapshot())
    }

    pub fn dispatch(&mut self, request: DispatchRequest) -> DispatchOutcome {
        // Invalid IDs are never recorded, so they cannot grow the idempotency ledger.
        if let Err(message) = validate_request_id(&request.request_id) {
            return DispatchOutcome::rejected(message);
        }
        let key = (request.account_id.clone(), request.request_id.clone());
        if let Some(recorded) = self.requests.get(&key) {
            return if recorded.request == request {
                recorded.outcome.clone()
            } else {
                DispatchOutcome::rejected("This request ID was already used for another payload.")
            };
        }
        let outcome = if let Err(message) = self.validate_request(&request) {
            DispatchOutcome::rejected(message)
        } else if self
            .uncertain_targets
            .contains_key(&DispatchTarget::from(&request))
        {
            DispatchOutcome::uncertain(
                "The previous start for this target is uncertain. Unlock it after checking, or reset the demo.",
            )
        } else {
            match self.scenario {
                DemoScenario::Offline => DispatchOutcome::rejected(
                    "Offline: the start was not sent. Reconnect before trying again.",
                ),
                DemoScenario::Forbidden => DispatchOutcome::rejected(
                    "Access denied: workflow write permission is required.",
                ),
                DemoScenario::Uncertain => {
                    self.uncertain_targets
                        .insert(DispatchTarget::from(&request), Utc::now().to_rfc3339());
                    DispatchOutcome::uncertain(
                        "Simulated connection loss after sending. The outcome is uncertain; do not retry.",
                    )
                }
                DemoScenario::Normal => self.create_run(&request),
            }
        };
        self.requests.insert(
            key,
            RecordedRequest {
                request,
                outcome: outcome.clone(),
            },
        );
        outcome
    }

    fn validate_request(&self, request: &DispatchRequest) -> Result<Inputs, String> {
        if !self
            .snapshot
            .accounts
            .iter()
            .any(|account| account.id == request.account_id)
        {
            return Err("Unknown account.".into());
        }
        let repo = self
            .snapshot
            .repos
            .iter()
            .find(|repo| repo.id == request.repo_id && repo.account_id == request.account_id)
            .ok_or_else(|| "Repository does not belong to this account.".to_string())?;
        if !repo.selected {
            return Err("This repository is not shown in the tray.".into());
        }
        if repo.permission != Permission::Write {
            return Err("Repository write permission is required.".into());
        }
        let workflow = self
            .snapshot
            .workflows
            .iter()
            .find(|workflow| workflow.repo_id == repo.id && workflow.id == request.workflow_id)
            .ok_or_else(|| "Unknown workflow for this repository.".to_string())?;
        // GitHub only dispatches workflows present with workflow_dispatch on the default branch.
        if !workflow.dispatch || !workflow.refs.contains(&repo.default_branch) {
            return Err("Workflow is not dispatchable on the default branch.".into());
        }
        if !workflow.refs.contains(&request.git_ref) {
            return Err("Choose a known branch or tag for this workflow.".into());
        }
        validate_inputs(&workflow.inputs, &request.inputs).map_err(|error| error.to_string())
    }

    fn create_run(&mut self, request: &DispatchRequest) -> DispatchOutcome {
        let workflow = self
            .snapshot
            .workflows
            .iter()
            .find(|workflow| {
                workflow.repo_id == request.repo_id && workflow.id == request.workflow_id
            })
            .expect("dispatch target was validated");
        let run_id = self.next_run_id;
        self.next_run_id += 1;
        let now = Utc::now().to_rfc3339();
        self.snapshot.runs.insert(
            0,
            Run {
                id: run_id,
                repo_id: request.repo_id.clone(),
                workflow_id: request.workflow_id,
                name: workflow.name.clone(),
                branch: request.git_ref.clone(),
                status: RunStatus::Queued,
                conclusion: None,
                created_at: now.clone(),
                updated_at: now.clone(),
                html_url: String::new(),
            },
        );
        self.snapshot.updated_at = now;
        self.simulated_runs.insert(
            run_id,
            SimulatedRun {
                started: Instant::now(),
                initial_status: RunStatus::Queued,
            },
        );
        DispatchOutcome {
            state: DispatchState::Accepted,
            run_id: Some(run_id),
            html_url: None,
            message: "Demo workflow queued locally. No real workflow was started.".into(),
        }
    }

    pub fn set_scenario(&mut self, scenario: String) -> Result<Snapshot, String> {
        self.scenario = scenario.parse()?;
        let (connectivity, message) = match self.scenario {
            DemoScenario::Normal => (Connectivity::Online, DEMO_MESSAGE),
            DemoScenario::Offline => (
                Connectivity::Offline,
                "Offline demo: showing cached data. Workflow starts are disabled.",
            ),
            DemoScenario::Forbidden => (
                Connectivity::Forbidden,
                "Forbidden demo: cached data remains visible; workflow starts are disabled.",
            ),
            DemoScenario::Uncertain => (
                Connectivity::Online,
                "Uncertain demo: the next start simulates a lost response. Retry stays blocked until reset.",
            ),
        };
        self.snapshot.connectivity = connectivity;
        self.snapshot.stale = connectivity != Connectivity::Online;
        self.snapshot.message = Some(message.into());
        Ok(self.snapshot())
    }

    pub fn reset(&mut self) -> Snapshot {
        *self = Self::new();
        self.snapshot()
    }
}

#[cfg(test)]
mod tests;
