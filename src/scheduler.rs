use dashmap::DashMap;
use priority_queue::DoublePriorityQueue;
use rand::seq::SliceRandom;
use serde::{Deserialize, Serialize};
use std::{
    error::Error,
    fmt,
    sync::{Arc, RwLock},
    vec,
};

use crate::elo;

#[derive(Debug)]
pub struct SchedulerError {
    details: String,
}

impl SchedulerError {
    pub fn new(msg: &str) -> SchedulerError {
        SchedulerError {
            details: msg.to_string(),
        }
    }
}

impl fmt::Display for SchedulerError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.details)
    }
}

impl Error for SchedulerError {
    fn description(&self) -> &str {
        &self.details
    }
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Item {
    pub id: String,
    pub name: String,
    pub location: String,
    pub description: String,
    pub score: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchPair {
    pub match_pair_id: String,
    pub i1: String,
    pub i2: String,
    visit_count: i32,
    winner: Option<MatchWinner>,
    judge_id: Option<String>,
}

#[derive(Clone, PartialEq, Debug, Deserialize, Serialize)]
pub struct Judge {
    id: String,
    email: String,
}

#[derive(Copy, Clone, PartialEq, Debug, Serialize, Deserialize)]
pub enum MatchWinner {
    A,
    B,
}

#[derive(Copy, Clone, PartialEq, Debug)]
pub enum States {
    NoState,
    Init,
    Continuous,
    End,
}

#[derive(Clone)]
pub struct SchedulerState {
    current_state: Arc<RwLock<States>>,
    judges: Arc<RwLock<Vec<Judge>>>,
    items: Arc<DashMap<String, Box<Item>>>,
    matches: Arc<DashMap<String, Arc<MatchPair>>>,
    mq: Arc<RwLock<DoublePriorityQueue<String, i32>>>,
}

fn create_initial_matches(competitors: &[Box<Item>], n: usize) -> Vec<MatchPair> {
    let mut matches: Vec<MatchPair> = vec![];
    let rng = &mut rand::thread_rng();
    for _ in 0..n {
        let mut cc: Vec<&Box<Item>> = Vec::from_iter(competitors);
        cc.shuffle(rng);
        if cc.len() % 2 != 0 {
            cc.insert(0, cc.get(0).unwrap());
        }
        for i in 0..(cc.len() / 2) {
            let c1 = *cc.get(i).unwrap();
            let c2 = *cc.get(cc.len() - 1 - i).unwrap();
            matches.push(MatchPair {
                match_pair_id: uuid::Uuid::new_v4().to_string(),
                i1: c1.id.clone(),
                i2: c2.id.clone(),
                visit_count: 0,
                winner: None,
                judge_id: None,
            });
        }
    }
    matches
}

impl Item {
    pub fn new(name: String, location: String, description: String) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name,
            location,
            description,
            score: elo::algo::INITIAL_ELO,
        }
    }
}

impl Judge {
    pub fn new(email: String) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            email,
        }
    }

    pub fn from_id(email: String, id: String) -> Self {
        Self { id, email }
    }

    pub fn log_match_action(&self) {
        println!("match logged");
    }
}

impl SchedulerState {
    pub fn new() -> SchedulerState {
        let current_state = Arc::from(RwLock::from(States::NoState));
        let judges = Arc::from(RwLock::from(vec![]));
        let items = Arc::from(DashMap::new());
        let matches = Arc::from(DashMap::new());
        let mq = Arc::from(RwLock::from(DoublePriorityQueue::new()));
        SchedulerState {
            current_state,
            judges,
            items,
            matches,
            mq,
        }
    }

    pub fn judge_match(&self, judge: &Judge, match_id: &str, winner: MatchWinner) -> bool {
        let matches = self.get_matches();
        let match_pair = match matches.get(match_id) {
            Some(data) => data.clone(),
            None => return false,
        };

        {
            let new = MatchPair {
                match_pair_id: match_pair.match_pair_id.clone(),
                i1: match_pair.i1.clone(),
                i2: match_pair.i2.clone(),
                visit_count: match_pair.visit_count,
                winner: Some(winner),
                judge_id: match_pair.judge_id.clone(),
            };
            matches.insert(new.match_pair_id.clone(), new.into());
        }

        judge.log_match_action();
        let binding = self.items.clone();
        let mut s1 = binding.get_mut(&match_pair.i1).unwrap();
        let mut s2 = binding.get_mut(&match_pair.i2).unwrap();
        return match winner {
            MatchWinner::A => {
                let s1_elo = s1.score;
                let s2_elo = s2.score;
                let (r1, r2) =
                    elo::algo::calculate(s1_elo, s2_elo, elo::algo::K, elo::algo::Winner::P1);
                s1.score = r1;
                s2.score = r2;
                true
            }
            MatchWinner::B => {
                let s1_elo = s1.score;
                let s2_elo = s2.score;
                let (r1, r2) =
                    elo::algo::calculate(s1_elo, s2_elo, elo::algo::K, elo::algo::Winner::P2);
                s1.score = r1;
                s2.score = r2;
                true
            }
        };
    }

    pub fn get_state(&self) -> States {
        let guard = self.current_state.clone();
        let state = guard.read().unwrap();
        *state
    }

    pub fn get_judges(&self) -> Vec<Judge> {
        let binding = self.judges.clone();
        let iter = binding.read().unwrap();
        let mut v: Vec<Judge> = vec![];
        for i in iter.iter() {
            v.push(i.clone());
        }
        v
    }

    pub fn get_items(&self) -> Vec<Box<Item>> {
        let iter = &self.items;
        let mut v: Vec<Box<Item>> = vec![];
        for i in iter.iter() {
            v.push(i.value().clone());
        }
        v
    }

    fn state_machine_internal_transition(&self) -> Result<States, Box<dyn Error + '_>> {
        let guard = self.current_state.clone();
        let mut state = guard.write().unwrap();

        match *state {
            States::NoState => Ok(States::NoState),
            States::Init => {
                let q = self.mq.read()?;
                let peek = q.peek_min();
                if let Some(p) = peek {
                    if *p.1 < 1 {
                        *state = States::Init;
                        return Ok(States::Init);
                    }
                    *state = States::Continuous;
                    Ok(States::Continuous)
                } else {
                    *state = States::Init;
                    Ok(States::Init)
                }
            }
            States::Continuous => Ok(States::Continuous),
            States::End => Ok(States::End),
        }
    }

    pub fn get_matches(&self) -> Arc<DashMap<String, Arc<MatchPair>>> {
        self.matches.clone()
    }

    pub fn get_match_pairs(&self) -> Result<Vec<Arc<MatchPair>>, Box<dyn Error>> {
        let mut matches: Vec<Arc<MatchPair>> = vec![];
        let dmap = self.get_matches();
        for entry in dmap.iter() {
            let val = entry.clone();
            matches.push(val.clone());
        }
        Ok(matches)
    }

    pub fn seed_start(&self, n: usize) -> bool {
        if self.get_state() != States::NoState {
            return false;
        }

        let matches = self.get_matches();

        let pq_binding = self.mq.clone();
        let mut pq = pq_binding.write().unwrap();

        let item_vec = self.get_items();

        let mut starter_matches = create_initial_matches(&item_vec, n);
        for m in starter_matches.drain(..) {
            pq.push(m.match_pair_id.clone(), m.visit_count);
            matches.insert(m.match_pair_id.clone(), Arc::from(m));
        }

        let state_binding = self.current_state.clone();
        let mut old_state = state_binding.write().unwrap();
        *old_state = States::Init;

        true
    }

    pub fn add_items(&self, new_items: Vec<Box<Item>>) {
        let items = &self.items;
        for item in new_items {
            let id: String = item.id.clone();
            items.insert(id, item);
        }
    }

    pub fn add_item(&self, item: Item) {
        let items = &self.items;
        let id: String = item.id.clone();
        items.insert(id, Box::new(item));
    }

    pub fn add_judge(&self, new_judge: Judge) {
        let binding = self.judges.clone();
        let mut judges = binding.write().unwrap();
        judges.push(new_judge);
    }

    pub fn add_judges(&self, new_judges: &mut Vec<Judge>) {
        let binding = self.judges.clone();
        let mut judges = binding.write().unwrap();
        judges.append(new_judges);
    }

    fn find_next_match(&self) -> Result<Arc<MatchPair>, Box<SchedulerError>> {
        let state = self.get_state();
        match state {
            States::NoState => Err(Box::new(SchedulerError::new(
                "Cannot get next match while in NONE state",
            ))),
            States::Init => self.get_from_queue(0).unwrap(),
            States::Continuous => self.get_continuous_stage(),
            States::End => Err(Box::new(SchedulerError::new(
                "Cannot get next match while in END state",
            ))),
        }
    }

    fn get_from_queue(&self, min_prio: i32) -> Option<Result<Arc<MatchPair>, Box<SchedulerError>>> {
        let q = self.mq.write().unwrap();
        let matches = self.get_matches();
        if let Some(best) = q.peek_min() {
            let key = best.0;
            let prio = *best.1;
            if min_prio < prio {
                return None;
            }
            if let Some(val) = matches.get(key) {
                Some(Ok(val.clone()))
            } else {
                Some(Err(Box::new(SchedulerError::new("Match key not found"))))
            }
        } else {
            Some(Err(Box::new(SchedulerError::new("Could not peek queue"))))
        }
    }

    fn get_continuous_stage(&self) -> Result<Arc<MatchPair>, Box<SchedulerError>> {
        if let Some(Ok(queue_item)) = self.get_from_queue(1) {
            return Ok(queue_item);
        }

        let rng = &mut rand::thread_rng();
        let items = self.get_items();

        let choices: Vec<Box<Item>> = items.choose_multiple(rng, 2).cloned().collect();
        // TODO: keep a sorted array of closest scores and then select best match

        let id = uuid::Uuid::new_v4().to_string();
        let m = MatchPair {
            match_pair_id: id.clone(),
            i1: choices[0].id.clone(),
            i2: choices[1].id.clone(),
            visit_count: 0,
            winner: None,
            judge_id: None,
        };

        let as_arc = Arc::from(m);

        let hm = self.get_matches();
        hm.insert(id, as_arc.clone());
        Ok(as_arc)
    }

    pub fn give_judge_next_match(
        &self,
        judge: &Judge,
    ) -> Result<Arc<MatchPair>, Box<dyn Error + '_>> {
        self.state_machine_internal_transition()?;
        let nm = self.find_next_match();
        match nm {
            Ok(m) => {
                let m_id = &m.match_pair_id;
                let mut q = self.mq.write().unwrap();
                q.change_priority_by(m_id, |i| *i += 1);
                let new = Arc::new(MatchPair {
                    match_pair_id: m_id.clone(),
                    i1: m.i1.clone(),
                    i2: m.i2.clone(),
                    visit_count: m.visit_count + 1,
                    winner: m.winner,
                    judge_id: Some(judge.id.clone()),
                });
                self.get_matches()
                    .insert(m.match_pair_id.clone(), new.clone());
                Ok(new)
            }
            Err(e) => Err(e),
        }
    }
}
