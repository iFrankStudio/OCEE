//! OCEE-0 P1: deterministic state transition engine.
//!
//! A caller supplies one external action (or driver NOOP) per tick.
//! No organism VM, protocol interpreter, or real network is present in P1.
//! The runner is NOT an autonomous agent and these tests are NOT E001 results.

#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

pub const ORGANISM_ID: u64 = 1;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
pub enum NodeId { A, B, C }

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Config {
    pub epoch_length: u64,
    pub message_delay: u64,
    pub max_message_bytes: usize,
    pub a_gas_per_epoch: u64,
    pub b_gas_per_epoch: u64,
    pub c_bandwidth_per_epoch: u64,
    pub a_storage_bytes: u64,
    pub b_storage_bytes: u64,
    pub seed: u64,
}
impl Default for Config {
    fn default() -> Self {
        Self {
            epoch_length: 1024, message_delay: 2, max_message_bytes: 1024,
            a_gas_per_epoch: 12_000, b_gas_per_epoch: 50_000,
            c_bandwidth_per_epoch: 64 * 1024,
            a_storage_bytes: 256 * 1024, b_storage_bytes: 512 * 1024,
            seed: 1,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct NodeState {
    pub gas_per_epoch: u64,
    pub gas_available: u64,
    pub storage_capacity: u64,
    pub bandwidth_per_epoch: u64,
    pub bandwidth_available: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Object {
    pub owner: u64,
    pub version: u64,
    pub bytes: Vec<u8>,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct OrganismHostState {
    pub id: u64,
    pub home: NodeId,
    pub storage_used: u64,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Message {
    pub id: u64,
    pub actor: u64,
    pub source: NodeId,
    pub destination: NodeId,
    pub bytes: Vec<u8>,
    pub sent_step: u64,
    pub deliver_step: u64,
    pub sequence_no: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum EventKind {
    ResourceChange { node: NodeId, gas_per_epoch: u64 },
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ScheduledEvent {
    pub due_step: u64,
    pub priority: i32,
    pub sequence_no: u64,
    pub kind: EventKind,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct WorldState {
    pub schema_version: u32,
    pub step: u64,
    pub last_granted_epoch: Option<u64>,
    pub rng_state: u64,
    pub config: Config,
    pub nodes: BTreeMap<NodeId, NodeState>,
    pub organisms: BTreeMap<u64, OrganismHostState>,
    pub objects: BTreeMap<String, Object>,
    pub mailboxes: BTreeMap<NodeId, Vec<Message>>,
    pub pending_deliveries: Vec<Message>,
    pub events: Vec<ScheduledEvent>,
    pub next_sequence: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum Action {
    Observe { actor: u64 },
    Read { actor: u64, key: String },
    Write { actor: u64, key: String, expected_version: Option<u64>, bytes: Vec<u8> },
    Send { actor: u64, destination: NodeId, bytes: Vec<u8> },
    Receive { actor: u64, max_count: usize },
    // P2 only: deliberately not implemented as a fake success in P1.
    Exec { actor: u64, program_hash: String },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum Code {
    Ok, Denied, NotFound, Conflict, Quota, Gas, Bandwidth,
    Empty, BadRequest, Unsupported,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ActionResult {
    pub code: Code,
    pub payload: Vec<u8>,
    pub gas_charged: u64,
    pub bandwidth_charged: u64,
}
impl ActionResult {
    fn fail(code: Code) -> Self {
        Self { code, payload: vec![], gas_charged: 0, bandwidth_charged: 0 }
    }
    fn ok(payload: Vec<u8>, gas: u64, bandwidth: u64) -> Self {
        Self { code: Code::Ok, payload, gas_charged: gas, bandwidth_charged: bandwidth }
    }
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct AuditRecord {
    pub step: u64,
    pub action: Option<Action>,
    pub result: ActionResult,
    pub state_hash: String,
    pub previous_record_hash: String,
    pub record_hash: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct World {
    pub state: WorldState,
    pub audit: Vec<AuditRecord>,
}

fn hash_bytes(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    digest.iter().map(|b| format!("{b:02x}")).collect()
}
fn canonical_hash<T: Serialize>(value: &T) -> String {
    hash_bytes(&serde_json::to_vec(value).expect("serializable world state"))
}

impl World {
    pub fn new(config: Config) -> Self {
        assert!(config.epoch_length > 0, "epoch_length must be positive");
        assert!(config.message_delay > 0, "delivery delay must be positive");
        let mut nodes = BTreeMap::new();
        nodes.insert(NodeId::A, NodeState {
            gas_per_epoch: config.a_gas_per_epoch, gas_available: 0,
            storage_capacity: config.a_storage_bytes,
            bandwidth_per_epoch: 0, bandwidth_available: 0,
        });
        nodes.insert(NodeId::B, NodeState {
            gas_per_epoch: config.b_gas_per_epoch, gas_available: 0,
            storage_capacity: config.b_storage_bytes,
            bandwidth_per_epoch: 0, bandwidth_available: 0,
        });
        nodes.insert(NodeId::C, NodeState {
            gas_per_epoch: 0, gas_available: 0,
            storage_capacity: 0,
            bandwidth_per_epoch: config.c_bandwidth_per_epoch,
            bandwidth_available: 0,
        });
        let mut organisms = BTreeMap::new();
        organisms.insert(ORGANISM_ID, OrganismHostState {
            id: ORGANISM_ID, home: NodeId::A, storage_used: 0,
        });
        Self {
            state: WorldState {
                schema_version: 1, step: 0, last_granted_epoch: None,
                rng_state: config.seed, config, nodes, organisms,
                objects: BTreeMap::new(), mailboxes: BTreeMap::new(),
                pending_deliveries: vec![], events: vec![], next_sequence: 0,
            },
            audit: vec![],
        }
    }

    pub fn schedule_event(&mut self, event: ScheduledEvent) {
        assert!(event.due_step >= self.state.step, "cannot schedule event in the past");
        self.state.events.push(event);
    }

    pub fn state_hash(&self) -> String { canonical_hash(&self.state) }

    pub fn snapshot_json(&self) -> Vec<u8> {
        serde_json::to_vec(self).expect("serializable snapshot")
    }

    pub fn from_snapshot(bytes: &[u8]) -> Result<Self, serde_json::Error> {
        serde_json::from_slice(bytes)
    }

    fn deliver_events(&mut self) {
        let k = self.state.step;
        let mut events = std::mem::take(&mut self.state.events);
        events.sort_by_key(|e| (e.due_step, e.priority, e.sequence_no));
        for ev in events {
            if ev.due_step <= k {
                match ev.kind {
                    EventKind::ResourceChange { node, gas_per_epoch } => {
                        if let Some(n) = self.state.nodes.get_mut(&node) {
                            n.gas_per_epoch = gas_per_epoch;
                        }
                    }
                }
            } else {
                self.state.events.push(ev);
            }
        }
        let mut pending = std::mem::take(&mut self.state.pending_deliveries);
        pending.sort_by_key(|m| (m.deliver_step, m.sequence_no));
        for m in pending {
            if m.deliver_step <= k {
                self.state.mailboxes.entry(m.destination).or_default().push(m);
            } else {
                self.state.pending_deliveries.push(m);
            }
        }
    }

    fn grant_epoch_if_needed(&mut self) {
        let epoch = self.state.step / self.state.config.epoch_length;
        if self.state.last_granted_epoch == Some(epoch) { return; }
        for node in self.state.nodes.values_mut() {
            node.gas_available = node.gas_per_epoch;
            node.bandwidth_available = node.bandwidth_per_epoch;
        }
        self.state.last_granted_epoch = Some(epoch);
    }

    fn valid_actor(&self, actor: u64) -> bool {
        self.state.organisms.contains_key(&actor)
    }

    fn spend_a_gas(&mut self, amount: u64) -> Result<(), Code> {
        let a = self.state.nodes.get_mut(&NodeId::A).expect("node A");
        if a.gas_available < amount { return Err(Code::Gas); }
        a.gas_available -= amount;
        Ok(())
    }

    fn apply_action(&mut self, action: Action) -> ActionResult {
        let actor = match &action {
            Action::Observe {actor} | Action::Read {actor,..} |
            Action::Write {actor,..} | Action::Send {actor,..} |
            Action::Receive {actor,..} | Action::Exec {actor,..} => *actor,
        };
        if !self.valid_actor(actor) { return ActionResult::fail(Code::Denied); }
        match action {
            Action::Observe {..} => {
                if let Err(code) = self.spend_a_gas(1) { return ActionResult::fail(code); }
                let a = self.state.nodes.get(&NodeId::A).expect("node A");
                let view = serde_json::to_vec(&(self.state.step,a.gas_available,a.storage_capacity))
                    .expect("observable view");
                ActionResult::ok(view, 1, 0)
            }
            Action::Read {key,..} => {
                let Some(obj) = self.state.objects.get(&key) else {
                    return ActionResult::fail(Code::NotFound);
                };
                if obj.owner != actor || !key.starts_with("/self/") {
                    return ActionResult::fail(Code::Denied);
                }
                let bytes = obj.bytes.clone();
                let gas = 1 + (bytes.len() as u64).saturating_add(63) / 64;
                if let Err(code) = self.spend_a_gas(gas) { return ActionResult::fail(code); }
                ActionResult::ok(bytes, gas, 0)
            }
            Action::Write {key,expected_version,bytes,..} => {
                if !key.starts_with("/self/") { return ActionResult::fail(Code::Denied); }
                let old = self.state.objects.get(&key).cloned();
                if let Some(ref object) = old {
                    if object.owner != actor { return ActionResult::fail(Code::Denied); }
                }
                let old_version = old.as_ref().map(|v| v.version);
                if old_version != expected_version { return ActionResult::fail(Code::Conflict); }
                let old_len = old.as_ref().map_or(0,|v|v.bytes.len() as u64);
                let used = self.state.organisms.get(&actor).expect("actor").storage_used;
                let Some(next_used) = used.checked_sub(old_len)
                    .and_then(|v| v.checked_add(bytes.len() as u64)) else {
                    return ActionResult::fail(Code::Quota);
                };
                let cap = self.state.nodes.get(&NodeId::A).expect("node A").storage_capacity;
                if next_used > cap { return ActionResult::fail(Code::Quota); }
                let gas = 2 + (bytes.len() as u64).saturating_add(15) / 16;
                if let Err(code) = self.spend_a_gas(gas) { return ActionResult::fail(code); }
                let version = old_version.map_or(0,|v|v+1);
                self.state.objects.insert(key,Object{owner:actor,version,bytes});
                self.state.organisms.get_mut(&actor).expect("actor").storage_used = next_used;
                ActionResult::ok(version.to_le_bytes().to_vec(),gas,0)
            }
            Action::Send {destination,bytes,..} => {
                if destination != NodeId::C { return ActionResult::fail(Code::Denied); }
                if bytes.len() > self.state.config.max_message_bytes {
                    return ActionResult::fail(Code::BadRequest);
                }
                let gas = 2 + (bytes.len() as u64).saturating_add(63) / 64;
                let bandwidth = bytes.len() as u64;
                let available = self.state.nodes.get(&NodeId::C).expect("node C")
                    .bandwidth_available;
                if available < bandwidth { return ActionResult::fail(Code::Bandwidth); }
                if let Err(code) = self.spend_a_gas(gas) { return ActionResult::fail(code); }
                self.state.nodes.get_mut(&NodeId::C).expect("node C")
                    .bandwidth_available -= bandwidth;
                let id = self.state.next_sequence;
                self.state.next_sequence += 1;
                let m = Message {
                    id, actor, source: NodeId::A, destination,
                    bytes, sent_step:self.state.step,
                    deliver_step:self.state.step + self.state.config.message_delay,
                    sequence_no:id,
                };
                self.state.pending_deliveries.push(m);
                ActionResult::ok(id.to_le_bytes().to_vec(),gas,bandwidth)
            }
            Action::Receive {max_count,..} => {
                if max_count == 0 || max_count > 64 { return ActionResult::fail(Code::BadRequest); }
                let count = self.state.mailboxes.get(&NodeId::A)
                    .map_or(0,|box_|box_.len().min(max_count));
                if count == 0 { return ActionResult::fail(Code::Empty); }
                if let Err(code) = self.spend_a_gas(1) { return ActionResult::fail(code); }
                let mailbox = self.state.mailboxes.entry(NodeId::A).or_default();
                let read: Vec<Message> = mailbox.drain(..count).collect();
                ActionResult::ok(serde_json::to_vec(&read).expect("mail"),1,0)
            }
            Action::Exec {..} => ActionResult::fail(Code::Unsupported),
        }
    }

    /// One deterministic world tick; with None, only the driver advances the world.
    /// For P1, Action is externally supplied, not chosen by an organism VM.
    pub fn tick(&mut self, action: Option<Action>) -> ActionResult {
        let k = self.state.step;
        self.deliver_events();
        self.grant_epoch_if_needed();
        let result = match action.clone() {
            Some(a) => self.apply_action(a),
            None => ActionResult::ok(vec![],0,0),
        };
        self.state.step = self.state.step.checked_add(1).expect("world step overflow");
        let state_hash = self.state_hash();
        let previous_record_hash = self.audit.last()
            .map_or_else(|| "GENESIS".to_owned(), |r| r.record_hash.clone());
        let record_hash = canonical_hash(&(
            k, &action, &result, &state_hash, &previous_record_hash
        ));
        self.audit.push(AuditRecord{
            step:k, action, result:result.clone(),
            state_hash, previous_record_hash, record_hash,
        });
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn event_is_applied_before_new_epoch_grant() {
        let mut w=World::new(Config::default());
        w.schedule_event(ScheduledEvent {
            due_step: 1024, priority:0,sequence_no:0,
            kind: EventKind::ResourceChange{node:NodeId::A,gas_per_epoch:4_000},
        });
        for _ in 0..1024 { w.tick(None); }
        assert_eq!(w.state.step,1024);
        let r=w.tick(Some(Action::Observe{actor:ORGANISM_ID}));
        assert_eq!(r.code,Code::Ok);
        assert_eq!(w.state.nodes[&NodeId::A].gas_available,3_999);
        assert_eq!(w.state.last_granted_epoch,Some(1));
    }

    #[test]
    fn c_message_is_delayed_and_b_cannot_be_addressed() {
        let mut w=World::new(Config::default());
        assert_eq!(w.tick(Some(Action::Send{
            actor:ORGANISM_ID,destination:NodeId::B,bytes:vec![1],
        })).code,Code::Denied);
        assert_eq!(w.tick(Some(Action::Send{
            actor:ORGANISM_ID,destination:NodeId::C,bytes:vec![42],
        })).code,Code::Ok);
        assert!(w.state.mailboxes.get(&NodeId::C).is_none());
        w.tick(None); // at step=2, message is still in flight
        assert!(w.state.mailboxes.get(&NodeId::C).is_none());
        w.tick(None); // at step=3, first delivery
        assert_eq!(w.state.mailboxes[&NodeId::C][0].bytes,vec![42]);
    }

    #[test]
    fn atomic_cas_and_storage_limit() {
        let mut w=World::new(Config{a_storage_bytes:2,..Config::default()});
        let write=|bytes:Vec<u8>,expected_version| Action::Write{
            actor:ORGANISM_ID,key:"/self/test".into(),expected_version,bytes
        };
        assert_eq!(w.tick(Some(write(vec![1,2],None))).code,Code::Ok);
        assert_eq!(w.tick(Some(write(vec![9],None))).code,Code::Conflict);
        assert_eq!(w.tick(Some(write(vec![1,2,3],Some(0)))).code,Code::Quota);
        assert_eq!(w.state.objects["/self/test"].bytes,vec![1,2]);
        assert_eq!(w.state.organisms[&ORGANISM_ID].storage_used,2);
    }

    #[test]
    fn identical_trace_produces_identical_hashes_and_snapshots() {
        let actions=[
            Some(Action::Observe{actor:ORGANISM_ID}),
            Some(Action::Write{actor:ORGANISM_ID,key:"/self/n".into(),
                expected_version:None,bytes:vec![1,2,3]}),
            Some(Action::Send{actor:ORGANISM_ID,destination:NodeId::C,
                bytes:vec![7]}),
            None,
            None,
        ];
        let mut a=World::new(Config::default());
        let mut b=World::new(Config::default());
        for action in actions {
            a.tick(action.clone());
            b.tick(action);
        }
        assert_eq!(a.state_hash(),b.state_hash());
        assert_eq!(a.audit,b.audit);
        let restored=World::from_snapshot(&a.snapshot_json()).unwrap();
        assert_eq!(a,restored);
        b.tick(Some(Action::Observe{actor:ORGANISM_ID}));
        assert_ne!(a.state_hash(),b.state_hash());
    }

    #[test]
    fn exhausting_gas_does_not_destroy_persistent_data() {
        let mut w=World::new(Config{a_gas_per_epoch:4,..Config::default()});
        let first=w.tick(Some(Action::Write{actor:ORGANISM_ID,
            key:"/self/v".into(),expected_version:None,bytes:vec![1]}));
        assert_eq!(first.code,Code::Ok); // cost=3
        let refused=w.tick(Some(Action::Write{actor:ORGANISM_ID,
            key:"/self/v".into(),expected_version:Some(0),bytes:vec![2]}));
        assert_eq!(refused.code,Code::Gas);
        assert_eq!(w.state.objects["/self/v"].bytes,vec![1]);
        assert_eq!(w.state.organisms[&ORGANISM_ID].storage_used,1);
    }
}
