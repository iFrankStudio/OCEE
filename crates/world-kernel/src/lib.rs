//! OCEE-0 P1: deterministic state transition engine.
//!
//! A caller supplies one external action (or driver NOOP) per tick.
//! No organism VM, protocol interpreter, or real network is present in P1.
//! The runner is NOT an autonomous agent and these tests are NOT E001 results.

#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, VecDeque};
use ocee_protocol_sim::{Family, ParseError, Service, ServiceOutput};

mod program_host;

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
    pub active_program_hash: Option<String>,
    pub vm_continuation: Option<ocee_organism_vm::Continuation>,
    pub pending_feedback: Option<Vec<u8>>,
    pub last_return: Option<ocee_organism_vm::Value>,
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
    pub protocol_service: Option<Service>,
    pub service_events: Vec<ServiceEvent>,
    /// S36 development-only interactive oracle budget. Persisted for replay.
    /// Older snapshots default to zero; unqueried world hashes stay stable.
    #[serde(default,skip_serializing_if="probe_count_is_zero")]
    pub probe_count: u32,
}
fn probe_count_is_zero(count:&u32)->bool{*count==0}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ServiceEvent {
    pub step: u64,
    pub request_message_id: u64,
    pub b_gas_charged: u64,
    pub response_message_id: u64,
    pub diagnostic: Option<ParseError>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct TransportDiagnostic {
    pub in_reply_to_message_id: u64,
    pub code: ParseError,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct HelloSample {
    pub published_step: u64,
    pub request: Vec<u8>,
    pub response: Vec<u8>,
}

/// Public workload data, never a prebuilt protocol frame or answer.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct WorkItem {
    pub job_id: u32,
    pub operation: String,
    pub count: u16,
    pub values: Vec<u32>,
    pub values_le: Vec<u8>,
    pub answer_schema: String,
}

/// Public input/output examples; hidden rule and held-out answer are excluded.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TransformExample { pub input: Vec<u8>, pub output: u32 }
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TransformTask {
    pub task_id: String,
    pub examples: Vec<TransformExample>,
    pub held_out_input: Vec<u8>,
}

/// S9 development-only external witness, emitted after a real faulty tool
/// execution. Two agreeing labels are informative; absence/disagreement
/// explicitly represents insufficient evidence. Not an E001 evaluator.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BoundaryWitness {
    pub task_id: String,
    pub input: Vec<u8>,
    pub count: u32,
    pub first: Option<u32>,
    pub second: Option<u32>,
    pub observed_after_step: u64,
    /// Declared source, not a cryptographic authentication proof.
    pub source_id: String,
    /// Bound to the exact registered immutable tool version that was probed.
    pub subject_program_hash: String,
    /// Client-proposed per-probe challenge; only a freshness binding,
    /// not an authenticated statement about origin or correctness.
    #[serde(default)]
    pub probe_challenge: Option<String>,
}

/// Reference semantics available to the world fixture/researcher, never to
/// the running VM. Unlike S6, the empty-input answer varies by world seed.
pub fn hidden_compose_boundary(seed:u64,input:&[u8])->u32 {
    if input.is_empty(){((seed/3)%2) as u32}
    else{hidden_compose(seed,input)}
}

/// Researcher-side reference only; never reachable by organism VM world calls.
/// S5 tests candidate *operator selection*, not discovery of new primitives.
pub fn hidden_transform(seed:u64,input:&[u8])->u32 {
    match seed % 3 {
        0 => input.len() as u32,
        1 => input.iter().fold(0u32,|acc,&b|acc ^ u32::from(b)),
        _ => u32::from(*input.first().unwrap_or(&0)),
    }
}

/// Independent researcher-side S6 composition reference. Not callable from VM.
pub fn hidden_compose(seed:u64,input:&[u8])->u32 {
    let index=(seed%18) as usize;
    let select=|n:usize|match n {
        0=>input.len() as u32,
        1=>input.iter().fold(0u32,|acc,&b|acc ^ u32::from(b)),
        _=>u32::from(*input.first().unwrap_or(&0)),
    };
    let left=select(index%3);
    let right=select((index/3)%3);
    if index<9 {left.wrapping_add(right)}else{left ^ right}
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum Action {
    Observe { actor: u64 },
    Read { actor: u64, key: String },
    Write { actor: u64, key: String, expected_version: Option<u64>, bytes: Vec<u8> },
    Send { actor: u64, destination: NodeId, bytes: Vec<u8> },
    Receive { actor: u64, max_count: usize },
    // P2 only: deliberately not implemented as a fake success in P1.
    Exec { actor: u64, program_hash: String, input: Vec<u8> },
    /// S36 explicit development query against a local labelled task fixture.
    /// Unlike Read, the organism chooses the actual input at runtime.
    Probe { actor: u64, input: Vec<u8> },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum Code {
    Ok, Denied, NotFound, Conflict, Quota, Gas, Bandwidth,
    Empty, BadRequest, Unsupported, InvalidCode, IsolatedSideEffect,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ActionResult {
    pub code: Code,
    pub payload: Vec<u8>,
    pub gas_charged: u64,
    pub vm_gas_charged: u64,
    pub bandwidth_charged: u64,
    /// Kernel-issued execution step of an attempted isolated EXEC. Absent for
    /// non-executed/gas-refused actions. U32-scoped development receipt;
    /// organisms must abstain if the logical step cannot be represented.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exec_step: Option<u32>,
}
impl ActionResult {
    fn fail(code: Code) -> Self {
        Self { code, payload: vec![], gas_charged: 0, vm_gas_charged: 0, bandwidth_charged: 0, exec_step: None }
    }
    fn ok(payload: Vec<u8>, gas: u64, bandwidth: u64) -> Self {
        Self { code: Code::Ok, payload, gas_charged: gas, vm_gas_charged: 0, bandwidth_charged: bandwidth, exec_step: None }
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
            active_program_hash: None, vm_continuation: None,
            pending_feedback: None, last_return: None,
        });
        Self {
            state: WorldState {
                schema_version: 1, step: 0, last_granted_epoch: None,
                rng_state: config.seed, config, nodes, organisms,
                objects: BTreeMap::new(), mailboxes: BTreeMap::new(),
                pending_deliveries: vec![], events: vec![], next_sequence: 0,
                protocol_service: None, service_events: vec![],
                probe_count: 0,
            },
            audit: vec![],
        }
    }

    /// Construct a self-contained simulated C/B service; never opens sockets.
    /// The protocol parameters are kept in privileged world state, not OBSERVE.
    pub fn with_protocol(config:Config,family:Family,seed:u64)->Self {
        let mut world=Self::new(config);
        world.state.protocol_service=Some(Service::new(family,seed));
        world
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

    /// At most eight C messages processed in one tick; requests remain ordered.
    /// If reply capacity is unavailable, leave the request in C for a later tick.
    /// Publish a bounded simulated external witness only after the registered
    /// tool really failed on empty input. No output is present before failure.
    /// Witness trust/quality is part of the environment, NOT organism skill.
    fn publish_boundary_witness(&mut self,actor:u64,hash:&str,input:&[u8],
        code:Code,observed_step:u64) {
        const PATH:&str="/public/E001/boundary-witness";
        if self.state.protocol_service.is_none() || actor!=ORGANISM_ID ||
            !input.is_empty() || code!=Code::InvalidCode ||
            self.state.objects.contains_key(PATH) {return;}
        let Some(registry)=self.state.objects.get("/self/tools/transform-v1") else {return;};
        if registry.owner!=actor || registry.bytes!=hash.as_bytes() {return;}
        let seed=self.state.config.seed;
        let correct=hidden_compose_boundary(seed,&[]);
        let mode=(seed/3)%4;
        let (count,first,second)=match mode {
            0|1=>(2,Some(correct),Some(correct)),
            2=>(0,None,None),           // missing evidence
            _=>(2,Some(0),Some(1)),   // conflicting observers
        };
        let evidence=BoundaryWitness{
            task_id:"binary-byte-compose-v1".into(),input:vec![],
            count,first,second,observed_after_step:observed_step,
            source_id:"sim-boundary/v1".into(),subject_program_hash:hash.into(),
            probe_challenge:self.state.objects.get("/self/repair/probe-nonce")
                .filter(|obj|obj.owner==actor)
                .and_then(|obj|String::from_utf8(obj.bytes.clone()).ok())
        };
        self.state.objects.insert(PATH.into(),Object{
            owner:0,version:0,bytes:serde_json::to_vec(&evidence)
                .expect("witness must serialize"),
        });
    }

    /// S10: an explicit actively requested semantic check. Unlike S9's
    /// fault-only witness, this fixture publishes following *any* isolated
    /// empty-input EXEC of the currently registered tool, even if it returned
    /// Ok with a semantically incorrect value. No report exists beforehand.
    /// The simulated labels are correlated and are NOT trusted identities.
    fn publish_semantic_witness(&mut self,actor:u64,hash:&str,input:&[u8],
        code:Code,observed_step:u64) {
        const PATH:&str="/public/E001/semantic-witness";
        if self.state.protocol_service.is_none() || actor!=ORGANISM_ID ||
            !matches!(code,Code::Ok|Code::InvalidCode) ||
            !input.is_empty() || self.state.objects.contains_key(PATH){return;}
        let Some(registry)=self.state.objects.get("/self/tools/transform-v1") else{return;};
        if registry.owner!=actor || registry.bytes!=hash.as_bytes(){return;}
        let seed=self.state.config.seed;
        let correct=hidden_compose_boundary(seed,input);
        let (count,first,second)=match (seed/3)%4 {
            0|1=>(2,Some(correct),Some(correct)),
            2=>(0,None,None),
            _=>(2,Some(0),Some(1)),
        };
        let evidence=BoundaryWitness{
            task_id:"binary-byte-compose-v1".into(),input:input.to_vec(),
            count,first,second,observed_after_step:observed_step,
            source_id:"sim-primary/v1".into(),subject_program_hash:hash.into(),
            probe_challenge:self.state.objects.get("/self/repair/probe-nonce")
                .filter(|obj|obj.owner==actor)
                .and_then(|obj|String::from_utf8(obj.bytes.clone()).ok()),
        };
        self.state.objects.insert(PATH.into(),Object{
            owner:0,version:0,bytes:serde_json::to_vec(&evidence)
                .expect("semantic witness must serialize"),
        });
    }

    /// S11 development-only second simulated checker. It uses its own
    /// reference expression rather than copying the first witness object.
    /// This is NOT proof of institutional/source independence: both checks
    /// share the researcher's world seed and execution authority.
    fn publish_secondary_audit(&mut self,actor:u64,hash:&str,input:&[u8],
        code:Code,observed_step:u64){
        const PATH:&str="/public/E001/semantic-audit";
        if self.state.protocol_service.is_none() || actor!=ORGANISM_ID ||
            !matches!(code,Code::Ok|Code::InvalidCode) ||
            !input.is_empty() || self.state.objects.contains_key(PATH){return;}
        let Some(registry)=self.state.objects.get("/self/tools/transform-v1") else{return;};
        if registry.owner!=actor || registry.bytes!=hash.as_bytes(){return;}
        // Independently implemented S11 reference for this fixture's empty
        // input contract, not derived from /semantic-witness contents.
        let independent_answer=((self.state.config.seed >> 0)/3 & 1) as u32;
        let evidence=BoundaryWitness{
            task_id:"binary-byte-compose-v1".into(),
            input:input.to_vec(),count:1,first:Some(independent_answer),
            second:None,observed_after_step:observed_step,
            source_id:"sim-secondary/v1".into(),subject_program_hash:hash.into(),
            probe_challenge:self.state.objects.get("/self/repair/probe-nonce")
                .filter(|obj|obj.owner==actor)
                .and_then(|obj|String::from_utf8(obj.bytes.clone()).ok())
        };
        self.state.objects.insert(PATH.into(),Object{
            owner:0,version:0,bytes:serde_json::to_vec(&evidence)
                .expect("secondary audit must serialize"),
        });
    }

    fn publish_compose_task(&mut self) {
        const PATH:&str="/public/E001/compose";
        if self.state.protocol_service.is_none() || self.state.objects.contains_key(PATH){return;}
        let seed=self.state.config.seed;
        let a=vec![13,7,11];
        let b=vec![18,9,4,6];
        let held_out=vec![7,8,9,10,3,1];
        let task=TransformTask{
            task_id:"binary-byte-compose-v1".into(),
            examples:vec![
                TransformExample{input:a.clone(),output:hidden_compose(seed,&a)},
                TransformExample{input:b.clone(),output:hidden_compose(seed,&b)},
            ],held_out_input:held_out,
        };
        self.state.objects.insert(PATH.into(),Object{
            owner:0,version:0,bytes:serde_json::to_vec(&task).expect("task"),
        });
    }

    fn publish_transform_task(&mut self) {
        const PATH:&str="/public/E001/transform";
        if self.state.protocol_service.is_none() || self.state.objects.contains_key(PATH){return;}
        let seed=self.state.config.seed;
        let x1=vec![17+(seed%19) as u8,3,8];
        let x2=vec![9+(seed%20) as u8,2,7,4];
        let held_out=vec![31+(seed%16) as u8,0,9,6,2];
        let task=TransformTask{
            task_id:"unary-byte-transform-v1".into(),
            examples:vec![
                TransformExample{input:x1.clone(),output:hidden_transform(seed,&x1)},
                TransformExample{input:x2.clone(),output:hidden_transform(seed,&x2)},
            ],held_out_input:held_out,
        };
        self.state.objects.insert(PATH.into(),Object{
            owner:0,version:0,bytes:serde_json::to_vec(&task).expect("task"),
        });
    }

    fn publish_work_item(&mut self) {
        const PATH:&str="/public/E001/job0";
        if self.state.protocol_service.is_none() || self.state.objects.contains_key(PATH){return;}
        let mut rng=self.state.config.seed ^ 0x4f43_4545_2d4a_4f42;
        let mut values=Vec::new();
        let mut values_le=Vec::new();
        for _ in 0..3 {
            rng=rng.wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            let v=(rng>>32) as u32;
            values.push(v);
            values_le.extend_from_slice(&v.to_le_bytes());
        }
        let work=WorkItem{
            job_id:1,operation:"FOLD32".into(),count:3,
            values,values_le,answer_schema:"u32le".into(),
        };
        self.state.objects.insert(PATH.into(),Object{
            owner:0,version:0,bytes:serde_json::to_vec(&work).expect("work item"),
        });
    }

    fn publish_hello_sample(&mut self) {
        if self.state.step % 256 != 0 {return;}
        let Some(service)=self.state.protocol_service.as_ref() else {return;};
        let (request,response)=service.public_hello_sample(self.state.step);
        let sample=HelloSample{published_step:self.state.step,request,response};
        self.state.objects.insert("/public/C/hello".into(),Object{
            owner:0,version:self.state.step / 256,
            bytes:serde_json::to_vec(&sample).expect("sample"),
        });
    }

    fn process_service_inbox(&mut self) {
        if self.state.protocol_service.is_none() {return;}
        let mut queue:VecDeque<Message>=self.state.mailboxes.remove(&NodeId::C)
            .unwrap_or_default().into();
        let mut deferred=Vec::<Message>::new();
        for _ in 0..8 {
            let Some(message)=queue.pop_front() else {break;};
            let available_b=self.state.nodes.get(&NodeId::B).expect("node B").gas_available;
            let mut trial=self.state.protocol_service.as_ref().expect("service").clone();
            let processed=trial.handle(message.actor,self.state.step,&message.bytes,available_b);
            let (payload,diagnostic)=match processed.output {
                ServiceOutput::Frame(bytes)=>(bytes,None),
                ServiceOutput::Diagnostic(code)=>(
                    serde_json::to_vec(&TransportDiagnostic{
                        in_reply_to_message_id:message.id,code,
                    }).expect("diagnostic"),Some(code)),
            };
            let bandwidth=payload.len() as u64;
            let left=self.state.nodes.get(&NodeId::C).expect("node C").bandwidth_available;
            if bandwidth>left || payload.len()>self.state.config.max_message_bytes {
                deferred.push(message);
                break;
            }
            // Commit service state, B gas, link budget and outbound message together.
            self.state.protocol_service=Some(trial);
            self.state.nodes.get_mut(&NodeId::B).expect("node B").gas_available -=
                processed.b_gas_charged;
            self.state.nodes.get_mut(&NodeId::C).expect("node C").bandwidth_available -= bandwidth;
            let id=self.state.next_sequence;
            self.state.next_sequence+=1;
            self.state.pending_deliveries.push(Message{
                id,actor:message.actor,source:NodeId::C,destination:NodeId::A,
                bytes:payload,sent_step:self.state.step,
                deliver_step:self.state.step.saturating_add(self.state.config.message_delay),
                sequence_no:id,
            });
            self.state.service_events.push(ServiceEvent{
                step:self.state.step,request_message_id:message.id,
                response_message_id:id,b_gas_charged:processed.b_gas_charged,
                diagnostic,
            });
        }
        deferred.extend(queue);
        if !deferred.is_empty(){self.state.mailboxes.insert(NodeId::C,deferred);}
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
            Action::Receive {actor,..} | Action::Exec {actor,..} |
            Action::Probe {actor,..} => *actor,
        };
        if !self.valid_actor(actor) { return ActionResult::fail(Code::Denied); }
        match action {
            Action::Observe {..} => {
                if let Err(code) = self.spend_a_gas(1) { return ActionResult::fail(code); }
                let a = self.state.nodes.get(&NodeId::A).expect("node A");
                let endpoint=if self.state.protocol_service.is_some(){Some("/public/C/hello")}else{None};
                let view = serde_json::to_vec(&(self.state.step,a.gas_available,a.storage_capacity,endpoint))
                    .expect("observable view");
                ActionResult::ok(view, 1, 0)
            }
            Action::Read {key,..} => {
                let Some(obj) = self.state.objects.get(&key) else {
                    return ActionResult::fail(Code::NotFound);
                };
                if key != "/public/C/hello" && key != "/public/E001/job0" &&
                    key != "/public/E001/transform" &&
                    key != "/public/E001/compose" &&
                    // S33 development fixture, explicit read-only public ACL.
                    key != "/public/E001/compose-depth" &&
                    // S35 explicit third public label, READ only.
                    key != "/public/E001/compose-counterexample" &&
                    key != "/public/E001/boundary-witness" &&
                    key != "/public/E001/semantic-witness" &&
                    key != "/public/E001/semantic-audit" &&
                    (obj.owner != actor || !key.starts_with("/self/")) {
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
                let active_switch = match program_host::preflight_write(
                    &self.state, actor, &key, &bytes, old.as_ref()
                ) {
                    Ok(value) => value,
                    Err(code) => return ActionResult::fail(code),
                };
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
                program_host::after_write(&mut self.state,actor,active_switch);
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
            Action::Exec {program_hash,input,..} => program_host::isolated_exec(self,actor,&program_hash,&input),
            Action::Probe {input,..} => {
                // Deliberately no arbitrary oracle: only this explicitly
                // provisioned S36 development task can answer a query.
                // Restricted lengths and one charged request per episode
                // make the process finite and all evidence audit-visible.
                let Some(task)=self.state.objects.get("/public/E001/compose-depth")
                    .and_then(|obj|serde_json::from_slice::<TransformTask>(&obj.bytes).ok())
                    else {return ActionResult::fail(Code::Denied)};
                if task.task_id!="s36-interactive-probe-v1" ||
                    self.state.protocol_service.is_none() {
                    return ActionResult::fail(Code::Denied);
                }
                if input.is_empty() || input.len()>8 {
                    return ActionResult::fail(Code::BadRequest);
                }
                if self.state.probe_count>=1 {
                    return ActionResult::fail(Code::Quota);
                }
                let gas=40+4*(input.len() as u64);
                if let Err(code)=self.spend_a_gas(gas){
                    return ActionResult::fail(code);
                }
                self.state.probe_count+=1;
                // Hidden only inside the world environment, not visible
                // to VM instructions or the publicly published task.
                let stages=((self.state.config.seed%54)/18) as u32;
                let output=hidden_compose(self.state.config.seed,&input)
                    .wrapping_add(stages.wrapping_mul(input.len() as u32));
                let example=TransformExample{input,output};
                ActionResult::ok(
                    serde_json::to_vec(&example).expect("probe receipt"),gas,0)
            },
        }
    }

    /// Externally driven P1 action or explicit driver NOOP.
    pub fn tick(&mut self, action: Option<Action>) -> ActionResult {
        self.tick_mode(action,None)
    }

    /// P2: execute the active organism program; it may yield at most one action
    /// and only the world kernel actually commits that action.
    pub fn tick_autonomous(&mut self, actor:u64) -> ActionResult {
        self.tick_mode(None,Some(actor))
    }

    fn tick_mode(&mut self, input:Option<Action>, organism:Option<u64>)->ActionResult {
        let k = self.state.step;
        self.deliver_events();
        self.grant_epoch_if_needed();
        self.publish_hello_sample();
        self.publish_work_item();
        self.publish_transform_task();
        self.publish_compose_task();
        self.process_service_inbox(); // deterministic service phase, before VM intent
        let (action, vm_used, no_action_code) = if let Some(actor)=organism {
            let state=self.prepare_organism_action(actor);
            (state.action,state.gas_used,state.status)
        } else {(input,0,Code::Ok)};
        let mut result = match action.clone() {
            Some(a) => self.apply_action(a),
            None if no_action_code==Code::Ok => ActionResult::ok(vec![],0,0),
            None => ActionResult::fail(no_action_code),
        };
        result.vm_gas_charged=vm_used;
        if matches!(action.as_ref(),Some(Action::Exec{..})) &&
            matches!(result.code,Code::Ok|Code::InvalidCode){
            result.exec_step=u32::try_from(k).ok();
        }
        if let Some(Action::Exec{actor,program_hash,input})=action.as_ref(){
            self.publish_boundary_witness(*actor,program_hash,input,result.code,k);
            self.publish_semantic_witness(*actor,program_hash,input,result.code,k);
            self.publish_secondary_audit(*actor,program_hash,input,result.code,k);
        }
        if let Some(actor)=organism {
            // When switching the active program, the previous continuation has
            // been invalidated by after_write; never deliver old feedback to it.
            if let Some(host)=self.state.organisms.get_mut(&actor) {
                if host.vm_continuation.as_ref().is_some_and(|c| c.awaiting.is_some())
                    && action.is_some() {
                    host.pending_feedback=Some(serde_json::to_vec(&result)
                        .expect("serializable action feedback"));
                }
            }
        }
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
    fn semantic_witness_requires_a_real_executed_tool_not_a_gas_refusal(){
        use ocee_organism_vm::{
            Block,Function,Instruction,Op,Param,Program,Terminator,Type,
        };
        let pure=Program{format_version:1,entry_function:"main".into(),
            functions:vec![Function{name:"main".into(),args:vec![Type::Bytes],
                returns:Type::U32,blocks:vec![Block{id:"entry".into(),
                    params:vec![Param{id:0,ty:Type::Bytes}],
                    instructions:vec![Instruction{id:1,op:Op::ConstU32(0)}],
                    terminator:Terminator::Return{value:1}
                }]}]};
        pure.validate().unwrap();
        let hash=pure.content_hash().unwrap();
        let mut world=World::with_protocol(Config::default(),Family::P0,5);
        assert_eq!(world.tick(Some(Action::Write{
            actor:ORGANISM_ID,key:format!("/self/versions/{hash}"),
            expected_version:None,bytes:pure.canonical_bytes()
        })).code,Code::Ok);
        assert_eq!(world.tick(Some(Action::Write{
            actor:ORGANISM_ID,key:"/self/tools/transform-v1".into(),
            expected_version:None,bytes:hash.as_bytes().to_vec()
        })).code,Code::Ok);
        world.state.nodes.get_mut(&NodeId::A).unwrap().gas_available=0;
        let refused=world.tick(Some(Action::Exec{
            actor:ORGANISM_ID,program_hash:hash,input:vec![]
        }));
        assert_eq!(refused.code,Code::Gas);
        assert!(!world.state.objects.contains_key("/public/E001/semantic-witness"),
            "a refused action must not leak the hidden output as a witness");
        assert!(!world.state.objects.contains_key("/public/E001/semantic-audit"),
            "a refused action must not leak the secondary simulated answer");
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
