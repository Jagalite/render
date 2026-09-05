use crate::{Error, Id, Result};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, VecDeque};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum State {
    Accepted,
    Queued,
    Running,
    CancelRequested,
    Succeeded,
    Failed,
    Cancelled,
}
impl State {
    pub fn terminal(self) -> bool {
        matches!(self, Self::Succeeded | Self::Failed | Self::Cancelled)
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Budget {
    pub max_wall_ms: u64,
    pub max_bytes: u64,
    pub max_samples: u32,
    pub max_output_bytes: u64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Job {
    pub id: Id,
    pub principal: String,
    pub revision: String,
    pub state: State,
    pub budget: Budget,
    pub artifacts: Vec<String>,
    pub diagnostic: Option<String>,
    #[serde(default)]
    pub render_input: Option<RenderInput>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RenderInput {
    pub snapshot: crate::document::Snapshot,
    pub settings: crate::render::Settings,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    pub sequence: u64,
    pub job: Id,
    pub state: State,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Jobs {
    pub jobs: BTreeMap<Id, Job>,
    queue: VecDeque<Id>,
    events: VecDeque<Event>,
    next_sequence: u64,
    pub queue_limit: usize,
    pub per_principal_limit: usize,
    pub event_retention: usize,
}
impl Default for Jobs {
    fn default() -> Self {
        Self {
            jobs: BTreeMap::new(),
            queue: VecDeque::new(),
            events: VecDeque::new(),
            next_sequence: 0,
            queue_limit: 64,
            per_principal_limit: 8,
            event_retention: 256,
        }
    }
}
impl Jobs {
    fn event(&mut self, id: Id, state: State) {
        self.events.push_back(Event {
            sequence: self.next_sequence,
            job: id,
            state,
        });
        self.next_sequence += 1;
        while self.events.len() > self.event_retention {
            self.events.pop_front();
        }
    }
    pub fn submit(&mut self, mut job: Job) -> Result<Id> {
        if self.jobs.contains_key(&job.id) {
            return Err(Error::new("duplicate_id", "job ID exists"));
        }
        if job.principal.is_empty()
            || job.budget.max_samples == 0
            || job.budget.max_wall_ms == 0
            || job.budget.max_bytes == 0
        {
            return Err(Error::new("job", "invalid principal or budget"));
        }
        if self.queue.len() >= self.queue_limit
            || self
                .jobs
                .values()
                .filter(|j| j.principal == job.principal && !j.state.terminal())
                .count()
                >= self.per_principal_limit
        {
            return Err(Error::new(
                "admission",
                "bounded queue/principal limit reached",
            ));
        }
        let id = job.id;
        job.state = State::Accepted;
        self.jobs.insert(id, job);
        self.event(id, State::Accepted);
        self.jobs.get_mut(&id).expect("inserted").state = State::Queued;
        self.queue.push_back(id);
        self.event(id, State::Queued);
        Ok(id)
    }
    pub fn start_next(&mut self) -> Option<Id> {
        while let Some(id) = self.queue.pop_front() {
            let job = self.jobs.get_mut(&id)?;
            if job.state != State::Queued {
                continue;
            }
            job.state = State::Running;
            self.event(id, State::Running);
            return Some(id);
        }
        None
    }
    pub fn cancel(&mut self, principal: &str, id: Id) -> Result<State> {
        let j = self
            .jobs
            .get_mut(&id)
            .ok_or_else(|| Error::new("not_found", "job"))?;
        if j.principal != principal {
            return Err(Error::new("permission", "job owner required"));
        }
        let next = match j.state {
            State::Queued | State::Accepted => State::Cancelled,
            State::Running => State::CancelRequested,
            s => s,
        };
        if next == j.state {
            return Ok(next);
        }
        j.state = next;
        if next == State::Cancelled {
            self.queue.retain(|queued| *queued != id);
        }
        self.event(id, next);
        Ok(next)
    }
    pub fn finish(&mut self, id: Id, result: Result<Vec<String>>) -> Result<State> {
        let j = self
            .jobs
            .get_mut(&id)
            .ok_or_else(|| Error::new("not_found", "job"))?;
        if !matches!(j.state, State::Running | State::CancelRequested) {
            return Err(Error::new("job_state", "job is not running"));
        }
        j.state = if j.state == State::CancelRequested {
            State::Cancelled
        } else {
            match result {
                Ok(a) => {
                    j.artifacts = a;
                    State::Succeeded
                }
                Err(e) => {
                    j.diagnostic = Some(e.to_string());
                    State::Failed
                }
            }
        };
        let state = j.state;
        self.event(id, state);
        Ok(state)
    }
    pub fn events_after(&self, cursor: Option<u64>) -> Result<Vec<Event>> {
        if let (Some(c), Some(first)) = (cursor, self.events.front())
            && c.saturating_add(1) < first.sequence
        {
            return Err(Error::new(
                "cursor_expired",
                "event cursor is outside retained history",
            ));
        }
        Ok(self
            .events
            .iter()
            .filter(|e| cursor.is_none_or(|c| e.sequence > c))
            .cloned()
            .collect())
    }
    pub fn recover_interrupted(&mut self) {
        let ids: Vec<_> = self
            .jobs
            .values()
            .filter(|j| matches!(j.state, State::Running | State::CancelRequested))
            .map(|j| j.id)
            .collect();
        for id in ids {
            let j = self.jobs.get_mut(&id).expect("collected");
            j.state = State::Failed;
            j.diagnostic = Some("host interrupted; explicit retry required".into());
            self.event(id, State::Failed);
        }
    }
}
