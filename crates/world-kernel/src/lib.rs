//! OCEE-0 P1: deterministic state transition engine.
//!
//! A caller supplies one external action (or driver NOOP) per tick.
//! No organism VM, protocol interpreter, or real network is present in P1.
//! The runner is NOT an autonomous agent and these tests are NOT E001 results.

#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use ocee_protocol_sim::{Family, ParseError, Service, ServiceOutput};

mod program_host;
mod transform_escrow;
pub use transform_escrow::TransformEscrow;

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
    /// Kernel-certified, paid reference summary. Invalidated on any
    /// real-world object/transport/root mutation; never organism-writable.
    #[serde(default,skip_serializing_if="Option::is_none")]
    pub version_ref_index:Option<VersionRefIndex>,
    /// Optional finite, independently endowed development payer.
    #[serde(default,skip_serializing_if="Option::is_none")]
    pub transform_escrow:Option<TransformEscrow>,
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
    /// Kernel-held provenance for the latest successful development query.
    /// Not writable via organism /self/ storage or visible through Read.
    #[serde(default,skip_serializing_if="Option::is_none")]
    pub probe_record: Option<ProbeRecord>,
    /// One narrowly-authorized second probe on a task revision. The
    /// renewed hash is immutable for this world and persisted in snapshots.
    #[serde(default,skip_serializing_if="Option::is_none")]
    pub probe_renewed_for: Option<String>,
    /// S39: a task version fence created by an organism using a genuinely
    /// issued environment receipt. Only the kernel can establish a fence.
    #[serde(default,skip_serializing_if="BTreeMap::is_empty")]
    pub task_fences: BTreeMap<u64,String>,
    /// Each pending C message carries an independent version requirement.
    /// The kernel checks it again at service consumption, not only SEND.
    #[serde(default,skip_serializing_if="BTreeMap::is_empty")]
    pub guarded_sends: BTreeMap<u64,String>,
    #[serde(default,skip_serializing_if="Vec::is_empty")]
    pub fence_drops: Vec<TaskFenceDrop>,
    /// S42A: most recent successful guarded C request per actor, persisted
    /// so stale drops cannot authorize recovery of a newer in-flight send.
    #[serde(default,skip_serializing_if="BTreeMap::is_empty")]
    pub last_guarded_send:BTreeMap<u64,u64>,
    /// S40: frozen first active root for one guarded, audited recovery.
    #[serde(default,skip_serializing_if="Option::is_none")]
    pub initial_root_hash:Option<String>,
    #[serde(default,skip_serializing_if="BTreeMap::is_empty")]
    pub guarded_restarts:BTreeMap<u64,u32>,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct VersionRefIndex {
    pub actor:u64,
    pub active_hash:Option<String>,
    pub installed:BTreeSet<String>,
    pub referenced:BTreeSet<String>,
    pub scanned_bytes:u64,
    pub built_step:u64,
    /// Paid full-scan, kernel-owned per-object source memberships.
    /// None denotes an older certificate that cannot authorize delta writes.
    #[serde(default,skip_serializing_if="Option::is_none")]
    pub object_refs:Option<BTreeMap<String,BTreeSet<String>>>,
    /// All transport refs are retained until another full scan; any SEND,
    /// RECEIVE, service mutation or message delivery invalidates the index.
    #[serde(default,skip_serializing_if="BTreeSet::is_empty")]
    pub transport_refs:BTreeSet<String>,
}
#[derive(Clone,Debug,Eq,PartialEq,Serialize,Deserialize)]
pub struct TaskFenceDrop {
    pub message_id:u64,
    pub actor:u64,
    pub step:u64,
    pub expected_hash:String,
    pub observed_hash:Option<String>,
}

fn probe_count_is_zero(count:&u32)->bool{*count==0}
fn credit_is_zero(amount:&u64)->bool{*amount==0}

/// Exact environment task bytes and exact issued receipt are bound together.
/// This is an in-kernel provenance guard, NOT a cryptographic public proof.
#[derive(Clone,Debug,Eq,PartialEq,Serialize,Deserialize)]
pub struct ProbeRecord {
    pub task_hash: String,
    pub receipt: Vec<u8>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ServiceEvent {
    pub step: u64,
    pub request_message_id: u64,
    pub b_gas_charged: u64,
    pub response_message_id: u64,
    pub diagnostic: Option<ParseError>,
    /// S43A.2: immutable hash of the exact kernel-produced full Message.
    /// Old snapshot entries without the hash cannot authorize a new reply
    /// confirmation; they remain readable for historical accounting.
    #[serde(default,skip_serializing_if="Option::is_none")]
    pub response_message_hash: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct TransportDiagnostic {
    pub in_reply_to_message_id: u64,
    pub code: ParseError,
}

/// S43A: epistemic, request-scoped state derived from kernel-owned events.
/// "CommittedB" means B gas was actually charged, NOT that the caller
/// received an ACK, or that an application-level RESULT was verified.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum RequestState { Unknown, InFlight, Dropped, HandledNoB, CommittedB }

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum ReplyLocation { InTransit, Mailbox, NotQueued }

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RequestOutcome {
    pub request_id:u64,
    pub state:RequestState,
    #[serde(skip_serializing_if="Option::is_none")]
    pub response_message_id:Option<u64>,
    #[serde(skip_serializing_if="Option::is_none")]
    pub b_gas_charged:Option<u64>,
    #[serde(skip_serializing_if="Option::is_none")]
    pub reply_location:Option<ReplyLocation>,
}

/// Confirmation is issued only when a real service response matching the
/// request has actually been returned by an audited RECEIVE to this actor.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ConfirmedReply {
    pub request_id:u64,
    pub response_message_id:u64,
    pub b_gas_charged:u64,
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
    /// New explicitly metered, source-aware atomic CAS for normal owned data.
    /// Legacy Write semantics and past cost ledgers remain unchanged.
    WriteIndexed { actor: u64, key: String, expected_version: Option<u64>, bytes: Vec<u8> },
    /// Atomically install one canonical immutable program and CAS-update
    /// one owned /self/tools registry. No version orphan on failure.
    PublishTool {
        actor: u64, registry_key: String, program_bytes: Vec<u8>,
        expected_version: Option<u64>,
    },
    /// A real charged scan creates a kernel-owned, snapshot-persisted
    /// reference certificate (not supplied or edited by the organism).
    IndexVersionRefs { actor:u64 },
    /// Conservatively reuse the still-valid certified summary for
    /// bounded retirement; a stale or missing index fails closed.
    ReclaimIndexedVersion { actor:u64, program_hash:String },
    /// Charged, conservative removal of one unreferenced owned immutable
    /// version. Existing live roots/registries/pending raw-hash refs block.
    ReclaimVersion {actor:u64, program_hash:String},
    Send { actor: u64, destination: NodeId, bytes: Vec<u8> },
    Receive { actor: u64, max_count: usize },
    // P2 only: deliberately not implemented as a fake success in P1.
    Exec { actor: u64, program_hash: String, input: Vec<u8> },
    /// S36 explicit development query against a local labelled task fixture.
    /// Unlike Read, the organism chooses the actual input at runtime.
    Probe { actor: u64, input: Vec<u8> },
    /// Cross-check a /self/ cached probe receipt against the immutable
    /// kernel-held issue record and CURRENT public task fingerprint.
    /// Never returns the hidden label. Validation consumes A resources.
    VerifyProbe { actor: u64, evidence: Vec<u8> },
    /// S38: authorize one extra paid query for the exact allowed v2 task.
    /// A genuine previously issued old receipt is mandatory.
    RenewProbe { actor: u64, evidence: Vec<u8> },
    /// S39: bind this actor's subsequent C sends to the task fingerprint
    /// that issued this actual receipt; must match the current task.
    ArmTaskFence { actor: u64, evidence: Vec<u8> },
    /// S40: scoped view of this actor's actually dropped remote C message.
    FenceStatus {actor:u64},
    /// One-shot request by the VM, not an automatic kernel retry.
    RestartRoot {actor:u64},
    /// S42B: organism-supplied exact u64 ID from the real SEND receipt.
    /// These calls cannot use actor-level "latest request" fallback.
    FenceStatusFor {actor:u64,request_id:u64},
    RestartRootFor {actor:u64,request_id:u64},
    /// S43A: paid epistemic query. Unknown is NOT proof of failure.
    RequestOutcomeFor {actor:u64,request_id:u64},
    /// S43A: bind the exact received Message JSON to kernel service
    /// request/response correspondence and an actual audited RECEIVE.
    ConfirmResponseFor {actor:u64,request_id:u64,reply:Vec<u8>},
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
    /// Actual spendable A transferred from a finite development escrow.
    #[serde(default,skip_serializing_if="credit_is_zero")]
    pub a_credited:u64,
}
impl ActionResult {
    fn fail(code: Code) -> Self {
        Self { code, payload: vec![], gas_charged: 0, vm_gas_charged: 0, bandwidth_charged: 0, exec_step:None,a_credited:0 }
    }
    fn ok(payload: Vec<u8>, gas: u64, bandwidth: u64) -> Self {
        Self { code: Code::Ok, payload, gas_charged: gas, vm_gas_charged: 0, bandwidth_charged: bandwidth, exec_step:None,a_credited:0 }
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
                probe_count: 0,probe_record:None,probe_renewed_for:None,
                task_fences:BTreeMap::new(),guarded_sends:BTreeMap::new(),
                fence_drops:vec![],last_guarded_send:BTreeMap::new(),
                initial_root_hash:None,guarded_restarts:BTreeMap::new(),
                version_ref_index:None,transform_escrow:None,
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
        if self.state.pending_deliveries.iter().any(|m|m.deliver_step<=k){
            self.state.version_ref_index=None;
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
        let earned=self.state.transform_escrow.as_ref().and_then(|e|e.earned_a);
        for (id,node) in self.state.nodes.iter_mut() {
            // Preserve finite earned A first. In the overflow edge case the
            // effective fresh grant is clipped, never the earned balance.
            node.gas_available = if *id==NodeId::A {
                node.gas_per_epoch.saturating_add(earned.unwrap_or(0))
            }else{node.gas_per_epoch};
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
        self.state.version_ref_index=None;
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
        self.state.version_ref_index=None;
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
        self.state.version_ref_index=None;
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
        self.state.version_ref_index=None;
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
        self.state.version_ref_index=None;
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
        self.state.version_ref_index=None;
        self.state.objects.insert(PATH.into(),Object{
            owner:0,version:0,bytes:serde_json::to_vec(&work).expect("work item"),
        });
    }

    fn publish_hello_sample(&mut self) {
        if self.state.step % 256 != 0 {return;}
        let Some(service)=self.state.protocol_service.as_ref() else {return;};
        let (request,response)=service.public_hello_sample(self.state.step);
        let sample=HelloSample{published_step:self.state.step,request,response};
        self.state.version_ref_index=None;
        self.state.objects.insert("/public/C/hello".into(),Object{
            owner:0,version:self.state.step / 256,
            bytes:serde_json::to_vec(&sample).expect("sample"),
        });
    }

    fn process_service_inbox(&mut self) {
        if self.state.protocol_service.is_none() {return;}
        if self.state.mailboxes.get(&NodeId::C)
            .is_some_and(|m|!m.is_empty()){
            self.state.version_ref_index=None;
        }
        let mut queue:VecDeque<Message>=self.state.mailboxes.remove(&NodeId::C)
            .unwrap_or_default().into();
        let mut deferred=Vec::<Message>::new();
        for _ in 0..8 {
            let Some(message)=queue.pop_front() else {break;};
            // One more integrity check at ACTUAL B-service consumption.
            // The task can change after a perfectly valid SEND, while the
            // message waits in the network queue. A stale guarded request
            // is never given to the protocol service or charged B gas.
            if let Some(expected)=self.state.guarded_sends.get(&message.id).cloned(){
                let observed=self.state.objects.get("/public/E001/compose-depth")
                    .map(|obj|hash_bytes(&obj.bytes));
                if observed.as_deref()!=Some(expected.as_str()) {
                    self.state.guarded_sends.remove(&message.id);
                    self.state.fence_drops.push(TaskFenceDrop{
                        message_id:message.id,actor:message.actor,
                        step:self.state.step,
                        expected_hash:expected,observed_hash:observed,
                    });
                    continue;
                }
            }
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
            // Consume the fence on the same successful service commit; keep
            // it intact when a request is deferred by bandwidth.
            self.state.guarded_sends.remove(&message.id);
            // Commit service state, B gas, link budget and outbound message together.
            self.state.protocol_service=Some(trial);
            self.state.nodes.get_mut(&NodeId::B).expect("node B").gas_available -=
                processed.b_gas_charged;
            self.state.nodes.get_mut(&NodeId::C).expect("node C").bandwidth_available -= bandwidth;
            let id=self.state.next_sequence;
            self.state.next_sequence+=1;
            let response=Message{
                id,actor:message.actor,source:NodeId::C,destination:NodeId::A,
                bytes:payload,sent_step:self.state.step,
                deliver_step:self.state.step.saturating_add(self.state.config.message_delay),
                sequence_no:id,
            };
            let response_message_hash=hash_bytes(&serde_json::to_vec(&response)
                .expect("kernel-created message hash"));
            self.state.pending_deliveries.push(response);
            self.state.service_events.push(ServiceEvent{
                step:self.state.step,request_message_id:message.id,
                response_message_id:id,b_gas_charged:processed.b_gas_charged,
                diagnostic,response_message_hash:Some(response_message_hash),
            });
        }
        deferred.extend(queue);
        if !deferred.is_empty(){self.state.mailboxes.insert(NodeId::C,deferred);}
    }

    fn valid_actor(&self, actor: u64) -> bool {
        self.state.organisms.contains_key(&actor)
    }

    fn spend_a_gas(&mut self, amount: u64) -> Result<(), Code> {
        let total=self.state.nodes[&NodeId::A].gas_available;
        if total < amount { return Err(Code::Gas); }
        if let Some(e)=self.state.transform_escrow.as_mut(){
            if let Some(earned)=e.earned_a{
                let quota=total.checked_sub(earned).ok_or(Code::BadRequest)?;
                // Expiring quota first. Failed spends mutate neither account.
                e.earned_a=Some(earned-amount.saturating_sub(quota));
            }
        }
        self.state.nodes.get_mut(&NodeId::A).expect("node A").gas_available=total-amount;
        Ok(())
    }

    /// Conservative S42A gate: a real drop must correspond to the LAST
    /// successfully sent guarded request, under the still-current task.
    /// The VM does not yet explicitly carry the request ID (S42B work).
    fn has_latest_recoverable_drop(&self,actor:u64)->bool{
        let Some(&latest)=self.state.last_guarded_send.get(&actor)
            else{return false;};
        // A missing task is not a valid new task revision. In particular,
        // None == None must never authorize a root restart.
        let Some(current)=self.state.objects.get("/public/E001/compose-depth")
            .map(|o|hash_bytes(&o.bytes)) else{return false;};
        self.state.fence_drops.iter().any(|d|
            d.actor==actor && d.message_id==latest &&
            d.observed_hash.as_deref()==Some(current.as_str()) &&
            current!=d.expected_hash)
    }

    /// S42B: a concrete dropped C request must match the VM's request ID.
    /// Never authorize recovery while *other* guarded requests from this
    /// actor are pending. Any B computation already committed anywhere in
    /// the single-actor E001 world conservatively blocks replay.
    fn has_explicit_recoverable_drop(&self,actor:u64,request_id:u64)->bool{
        let Some(current)=self.state.objects.get("/public/E001/compose-depth")
            .map(|o|hash_bytes(&o.bytes)) else{return false;};
        if !self.state.fence_drops.iter().any(|d|
            d.actor==actor && d.message_id==request_id &&
            d.observed_hash.as_deref()==Some(current.as_str()) &&
            d.expected_hash!=current){return false;}
        let other_pending=self.state.pending_deliveries.iter()
            .chain(self.state.mailboxes.get(&NodeId::C).into_iter().flatten())
            .any(|m|m.actor==actor && m.id!=request_id &&
                self.state.guarded_sends.contains_key(&m.id));
        if other_pending {return false;}
        !self.state.service_events.iter().any(|e|e.b_gas_charged>0)
    }


    /// Bound the provenance lookup so audit growth cannot make a single
    /// organism query unmetered in host CPU. Older evidence degrades to
    /// Unknown/Conflict, never to fabricated success or safe retry.
    const REQUEST_PROVENANCE_WINDOW:usize=4096;

    fn recently_sent_by(&self,actor:u64,request_id:u64)->bool{
        let receipt=request_id.to_le_bytes();
        self.audit.iter().rev().take(Self::REQUEST_PROVENANCE_WINDOW).any(|a|
            a.result.code==Code::Ok &&
            a.result.payload.as_slice()==receipt.as_slice() &&
            matches!(&a.action,Some(Action::Send{
                actor:sender,destination:NodeId::C,..}) if *sender==actor))
    }

    fn request_outcome(&self,actor:u64,request_id:u64)->RequestOutcome{
        let mut out=RequestOutcome{request_id,state:RequestState::Unknown,
            response_message_id:None,b_gas_charged:None,reply_location:None};
        if !self.recently_sent_by(actor,request_id){return out;}
        if let Some(e)=self.state.service_events.iter()
            .rev().take(Self::REQUEST_PROVENANCE_WINDOW)
            .find(|e|e.request_message_id==request_id){
            out.state=if e.b_gas_charged>0{
                RequestState::CommittedB
            }else{RequestState::HandledNoB};
            out.response_message_id=Some(e.response_message_id);
            out.b_gas_charged=Some(e.b_gas_charged);
            out.reply_location=Some(
                if self.state.pending_deliveries.iter().any(|m|
                    m.id==e.response_message_id && m.actor==actor &&
                    m.source==NodeId::C && m.destination==NodeId::A){
                    ReplyLocation::InTransit
                }else if self.state.mailboxes.get(&NodeId::A)
                    .into_iter().flatten().any(|m|
                        m.id==e.response_message_id && m.actor==actor &&
                        m.source==NodeId::C && m.destination==NodeId::A){
                    ReplyLocation::Mailbox
                }else{ReplyLocation::NotQueued}
            );
        }else if self.state.fence_drops.iter()
            .rev().take(Self::REQUEST_PROVENANCE_WINDOW)
            .any(|d|d.actor==actor && d.message_id==request_id){
            out.state=RequestState::Dropped;
        }else if self.state.pending_deliveries.iter()
            .chain(self.state.mailboxes.get(&NodeId::C).into_iter().flatten())
            .any(|m|m.actor==actor && m.id==request_id &&
                m.destination==NodeId::C){
            out.state=RequestState::InFlight;
        }
        out
    }

    /// A service event alone is insufficient: the message must actually
    /// have appeared in an authenticated RECEIVE result for this actor.
    fn confirmed_response(&self,actor:u64,request_id:u64,reply:&[u8])
        ->Option<ConfirmedReply>{
        if !self.recently_sent_by(actor,request_id){return None;}
        let message=serde_json::from_slice::<Message>(reply).ok()?;
        if message.actor!=actor || message.source!=NodeId::C ||
            message.destination!=NodeId::A {return None;}
        let event=self.state.service_events.iter()
            .rev().take(Self::REQUEST_PROVENANCE_WINDOW)
            .find(|e|e.request_message_id==request_id &&
                e.response_message_id==message.id)?;
        // Requiring both an actual audited RECEIVE and the C-authored
        // complete message fingerprint closes the same-ID/different-body
        // spoof: a forged mailbox entry is not a service-authored reply.
        if message.deliver_step>self.state.step {return None;}
        let actual_hash=hash_bytes(&serde_json::to_vec(&message).ok()?);
        if event.response_message_hash.as_deref()!=Some(actual_hash.as_str()){
            return None;
        }
        let observed=self.audit.iter().rev()
            .take(Self::REQUEST_PROVENANCE_WINDOW).any(|a|{
                if a.result.code!=Code::Ok ||
                    !matches!(&a.action,Some(Action::Receive{
                        actor:receiver,..}) if *receiver==actor){
                    return false;
                }
                serde_json::from_slice::<Vec<Message>>(&a.result.payload)
                    .is_ok_and(|items|items.iter().any(|m|m==&message))
            });
        if !observed{return None;}
        Some(ConfirmedReply{request_id,
            response_message_id:message.id,b_gas_charged:event.b_gas_charged})
    }

    fn apply_action(&mut self, action: Action) -> ActionResult {
        let explicit_request=match &action {
            Action::FenceStatusFor{request_id,..} |
            Action::RestartRootFor{request_id,..}=>Some(*request_id),
            _=>None,
        };
        let actor = match &action {
            Action::Observe {actor} | Action::Read {actor,..} |
            Action::Write {actor,..} | Action::WriteIndexed{actor,..} |
            Action::PublishTool{actor,..} |
            Action::ReclaimVersion{actor,..} |
            Action::IndexVersionRefs{actor} |
            Action::ReclaimIndexedVersion{actor,..} | Action::Send {actor,..} |
            Action::Receive {actor,..} | Action::Exec {actor,..} |
            Action::Probe {actor,..} | Action::VerifyProbe{actor,..} |
            Action::RenewProbe{actor,..} | Action::ArmTaskFence{actor,..} |
            Action::FenceStatus{actor} | Action::RestartRoot{actor} |
            Action::FenceStatusFor{actor,..} |
            Action::RestartRootFor{actor,..} |
            Action::RequestOutcomeFor{actor,..} |
            Action::ConfirmResponseFor{actor,..} => *actor,
        };
        if !self.valid_actor(actor) { return ActionResult::fail(Code::Denied); }
        // The certified raw-hash scanner examines each object key and body
        // SEPARATELY in 64-byte windows. A short ordinary own-object Write
        // cannot introduce any new raw 64-byte program-hash reference.
        // Keeping the old 'referenced' SUPERSET is safe even if the Write
        // removes an old reference: false positives remain conservative.
        // Active, immutable-version and tool-registry mutations never qualify.
        // Charge the proven-short preservation path one extra A on success.
        let preserve_certified_short_write=match &action{
            Action::Write{key,bytes,..}=>self.state.version_ref_index.as_ref()
                .is_some_and(|index|
                    index.actor==actor &&
                    index.active_hash==self.state.organisms[&actor].active_program_hash &&
                    key.starts_with("/self/") &&
                    key.len()<64 && bytes.len()<64 &&
                    key!="/self/active" &&
                    !key.starts_with("/self/versions/") &&
                    !key.starts_with("/self/tools/")
                ),
            _=>false,
        };
        // Other possible reference mutations retain the original conservative
        // fail-closed *attempt* invalidation, including failed large writes.
        if (matches!(&action,Action::Write{..}) &&
                !preserve_certified_short_write) ||
            matches!(&action,Action::PublishTool{..}|
                Action::Send{..}|Action::Receive{..}|Action::Exec{..}|
                Action::RestartRoot{..}|Action::RestartRootFor{..}) {
            self.state.version_ref_index=None;
        }
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
                if let Some(suffix)=key.strip_prefix("/sys/transform-summary/") {
                    return self.read_transform_summary(actor,suffix);
                }
                if let Some(suffix)=key.strip_prefix("/sys/transform-price/") {
                    return self.read_transform_price(actor,suffix,false);
                }
                if let Some(suffix)=key.strip_prefix("/sys/transform-price-u16/") {
                    return self.read_transform_price(actor,suffix,true);
                }
                // Development-only, paid, owner-only immutable program catalog.
                // A catalog lists installed SELF versions, not garbage-safe
                // candidates. Only the real kernel ReclaimIndexedVersion may
                // decide whether a reported hash is currently reclaimable.
                if let Some(raw_page)=key.strip_prefix("/sys/self-version-catalog/"){
                    let page=match raw_page.parse::<u32>(){
                        Ok(p) if p<=127 && p.to_string()==raw_page=>p,
                        _=>return ActionResult::fail(Code::BadRequest),
                    };
                    let mut versions=Vec::<(u64,String)>::new();
                    let mut scanned_key_bytes=0u64;
                    for (object_key,object) in &self.state.objects {
                        let Some(hash)=object_key.strip_prefix("/self/versions/")
                            else{continue};
                        if object.owner!=actor || hash.len()!=64 ||
                            !hash.bytes().all(|b|b.is_ascii_digit() ||
                                (b'a'..=b'f').contains(&b)){
                            continue;
                        }
                        scanned_key_bytes=match scanned_key_bytes
                            .checked_add(object_key.len() as u64){
                            Some(n)=>n,None=>return ActionResult::fail(Code::Quota)
                        };
                        if u32::try_from(object.bytes.len()).is_err(){
                            return ActionResult::fail(Code::Quota);
                        }
                        versions.push((object.bytes.len() as u64,hash.into()));
                        if versions.len()>1024 {
                            return ActionResult::fail(Code::Quota);
                        }
                    }
                    // Prefer cheaper physical reclaim candidates without
                    // exposing active/registered/ref-count protection status.
                    versions.sort();
                    const PAGE_SIZE:usize=8;
                    let start=page as usize * PAGE_SIZE;
                    let count=versions.len().saturating_sub(start).min(PAGE_SIZE);
                    let entries=versions.iter().skip(start).take(count)
                        .map(|(bytes,hash)|serde_json::json!({
                            "hash":hash,"bytes":bytes
                        })).collect::<Vec<_>>();
                    let payload=serde_json::to_vec(&serde_json::json!({
                        "page":page,"count":count,"total":versions.len(),
                        "entries":entries
                    })).expect("version catalog metadata serializable");
                    let scanned=match scanned_key_bytes
                        .checked_add(payload.len() as u64) {
                        Some(n)=>n,None=>return ActionResult::fail(Code::Quota)
                    };
                    let gas=2+scanned.div_ceil(64);
                    if let Err(code)=self.spend_a_gas(gas){
                        return ActionResult::fail(code);
                    }
                    return ActionResult::ok(payload,gas,0);
                }
                // Development-only, atomic own-object CAS observation:
                // strictly map canonical suffixes to the requesting actor's
                // existing /self/ data. No public, system, or cross-actor
                // capabilities are exposed through this metadata path.
                // Generic paid own-object prefix: no task/receipt interpretation.
                if let Some(raw)=key.strip_prefix("/sys/self-prefix/"){
                    let Some((n,relative))=raw.split_once('/') else{
                        return ActionResult::fail(Code::BadRequest);
                    };
                    let n=match n.parse::<usize>(){
                        Ok(v) if (1..=256).contains(&v)&&v.to_string()==n=>v,
                        _=>return ActionResult::fail(Code::BadRequest),
                    };
                    if relative.is_empty()||relative.split('/').any(|p|p.is_empty()||p=="."||p==".."){
                        return ActionResult::fail(Code::BadRequest);
                    }
                    let Some(obj)=self.state.objects.get(&format!("/self/{relative}")) else{
                        return ActionResult::fail(Code::NotFound);
                    };
                    if obj.owner!=actor{return ActionResult::fail(Code::Denied);}
                    let Ok(size)=u32::try_from(obj.bytes.len()) else{
                        return ActionResult::fail(Code::Quota);
                    };
                    let mut payload=size.to_le_bytes().to_vec();
                    payload.extend_from_slice(&obj.bytes[..n.min(obj.bytes.len())]);
                    let fee=2+(payload.len() as u64).saturating_add(63)/64;
                    if let Err(code)=self.spend_a_gas(fee){return ActionResult::fail(code);}
                    return ActionResult::ok(payload,fee,0);
                }
                if let Some(relative)=key.strip_prefix("/sys/self-object/"){
                    if relative.is_empty() || relative.split('/').any(|part|
                        part.is_empty() || part=="." || part==".."){
                        return ActionResult::fail(Code::BadRequest);
                    }
                    let self_key=format!("/self/{relative}");
                    let Some(object)=self.state.objects.get(&self_key) else{
                        return ActionResult::fail(Code::NotFound);
                    };
                    if object.owner!=actor {
                        return ActionResult::fail(Code::Denied);
                    }
                    if u32::try_from(object.version).is_err(){
                        return ActionResult::fail(Code::Quota);
                    }
                    let version=object.version;
                    let bytes=object.bytes.clone();
                    let fee=2+(bytes.len() as u64).saturating_add(63)/64;
                    if let Err(code)=self.spend_a_gas(fee){
                        return ActionResult::fail(code);
                    }
                    let payload=serde_json::to_vec(&serde_json::json!({
                        "version":version,"bytes":bytes
                    })).expect("own-object snapshot serializable");
                    return ActionResult::ok(payload,fee,0);
                }
                // Development-only own-resource sensor, not a hidden task
                // oracle or a privileged version-list/read capability.
                // Values are exact real kernel state, constrained to the
                // VM's U32 typed observation range (never truncated).
                if key=="/sys/resource-status" {
                    let used=self.state.organisms[&actor].storage_used;
                    let cap=self.state.nodes[&NodeId::A].storage_capacity;
                    let balance=self.state.nodes[&NodeId::A].gas_available;
                    if u32::try_from(used).is_err() ||
                        u32::try_from(cap).is_err() ||
                        u32::try_from(balance).is_err() {
                        return ActionResult::fail(Code::Quota);
                    }
                    let gas=2;
                    if let Err(code)=self.spend_a_gas(gas) {
                        return ActionResult::fail(code);
                    }
                    let left=self.state.nodes[&NodeId::A].gas_available;
                    let payload=serde_json::to_vec(&serde_json::json!({
                        "used":used,"capacity":cap,"a_available":left
                    })).expect("own resources serializable");
                    return ActionResult::ok(payload,gas,0);
                }
                // S40: ordinary metered per-actor metadata. This isn't a
                // writable object, and exposes no code or secret data.
                // It permits a restarted VM to use correct monotonic CAS
                // rather than assuming /self/active always has version 0.
                if key=="/sys/active-version" {
                    let Some(obj)=self.state.objects.get("/self/active") else{
                        return ActionResult::fail(Code::NotFound);
                    };
                    if obj.owner!=actor{return ActionResult::fail(Code::Denied);}
                    let payload=serde_json::to_vec(
                        &serde_json::json!({"version":obj.version}))
                        .expect("active version response");
                    if let Err(code)=self.spend_a_gas(1){return ActionResult::fail(code);}
                    return ActionResult::ok(payload,1,0);
                }
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
                let base_gas = 2 + (bytes.len() as u64).saturating_add(15) / 16;
                let gas=base_gas + u64::from(preserve_certified_short_write);
                if let Err(code) = self.spend_a_gas(gas) { return ActionResult::fail(code); }
                let version = old_version.map_or(0,|v|v+1);
                self.state.objects.insert(key,Object{owner:actor,version,bytes});
                self.state.organisms.get_mut(&actor).expect("actor").storage_used = next_used;
                program_host::after_write(&mut self.state,actor,active_switch);
                ActionResult::ok(version.to_le_bytes().to_vec(),gas,0)
            }
            Action::WriteIndexed{key,bytes,expected_version,..}=>{
                program_host::write_certified_object(
                    self,actor,key,bytes,expected_version
                )
            }
            Action::PublishTool{registry_key,program_bytes,expected_version,..}=>{
                program_host::atomic_publish_tool(
                    self,actor,registry_key,program_bytes,expected_version
                )
            }
            Action::ReclaimVersion{program_hash,..}=>{
                program_host::reclaim_owned_version(self,actor,&program_hash)
            }
            Action::IndexVersionRefs{..}=>{
                program_host::certify_version_references(self,actor)
            }
            Action::ReclaimIndexedVersion{program_hash,..}=>{
                program_host::reclaim_certified_version(self,actor,&program_hash)
            }
            Action::Send {destination,bytes,..} => {
                if destination != NodeId::C { return ActionResult::fail(Code::Denied); }
                // S39 policy applies only once the organism itself has
                // established a fence from its real cached probe evidence.
                // Legacy unarmed worlds retain identical SEND behavior.
                let bound=self.state.task_fences.get(&actor).cloned();
                if let Some(ref expected)=bound {
                    let current=self.state.objects.get("/public/E001/compose-depth")
                        .map(|obj|hash_bytes(&obj.bytes));
                    if current.as_deref()!=Some(expected.as_str()){
                        return ActionResult::fail(Code::Conflict);
                    }
                }
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
                if let Some(task_hash)=bound {
                    self.state.guarded_sends.insert(id,task_hash);
                    self.state.last_guarded_send.insert(actor,id);
                }
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
            Action::RequestOutcomeFor{request_id,..}=>{
                // A status query costs gas even when the request is unknown.
                let gas=2;
                if let Err(code)=self.spend_a_gas(gas){return ActionResult::fail(code);}
                let result=self.request_outcome(actor,request_id);
                ActionResult::ok(serde_json::to_vec(&result).expect("request outcome"),gas,0)
            },
            Action::ConfirmResponseFor{request_id,reply,..}=>{
                if reply.len()>4096 {return ActionResult::fail(Code::BadRequest);}
                let gas=2+(reply.len() as u64).saturating_add(63)/64;
                if let Err(code)=self.spend_a_gas(gas){return ActionResult::fail(code);}
                let is_valid_message=serde_json::from_slice::<Message>(&reply).is_ok();
                let confirmation=self.confirmed_response(actor,request_id,&reply);
                let code=if confirmation.is_some(){Code::Ok}
                    else if !is_valid_message{Code::BadRequest}
                    else{Code::Conflict};
                ActionResult{code,
                    payload:confirmation.map(|v|serde_json::to_vec(&v)
                        .expect("confirmed reply")).unwrap_or_default(),
                    gas_charged:gas,vm_gas_charged:0,bandwidth_charged:0,
                    exec_step:None,a_credited:0}
            },
            Action::Probe {input,..} => {
                // Deliberately no arbitrary oracle: only this explicitly
                // provisioned S36 development task can answer a query.
                // Restricted lengths and one charged request per episode
                // make the process finite and all evidence audit-visible.
                let Some((task,task_hash))=self.state.objects
                    .get("/public/E001/compose-depth")
                    .and_then(|obj|serde_json::from_slice::<TransformTask>(&obj.bytes)
                        .ok().map(|t|(t,hash_bytes(&obj.bytes))))
                    else {return ActionResult::fail(Code::Denied)};
                if task.task_id!="s36-interactive-probe-v1" &&
                    task.task_id!="s37-changed-rule-v2" ||
                    self.state.protocol_service.is_none() {
                    return ActionResult::fail(Code::Denied);
                }
                if input.is_empty() || input.len()>8 {
                    return ActionResult::fail(Code::BadRequest);
                }
                // A successful first v1 query is not enough for v2:
                // an explicit, one-shot authorization must be granted,
                // bound to precisely the current v2 task fingerprint.
                if self.state.probe_count>=1 &&
                    !(self.state.probe_count==1 &&
                        self.state.probe_renewed_for.as_deref()==
                            Some(task_hash.as_str())) {
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
                let output=if task.task_id=="s37-changed-rule-v2"{
                    // Independent researcher-controlled task rule revision.
                    (input.len() as u32).wrapping_mul(2)
                }else{
                    hidden_compose(self.state.config.seed,&input)
                        .wrapping_add(stages.wrapping_mul(input.len() as u32))
                };
                let example=TransformExample{input,output};
                let issued=serde_json::to_vec(&example)
                    .expect("probe receipt");
                self.state.probe_record=Some(ProbeRecord{
                    task_hash,receipt:issued.clone()
                });
                ActionResult::ok(issued,gas,0)
            },
            Action::FenceStatus {..} | Action::FenceStatusFor{..} => {
                // S41: both positive and negative failure checks have a
                // bounded, visible price. Empty polling must not be free.
                let gas=1;
                if let Err(code)=self.spend_a_gas(gas){return ActionResult::fail(code);}
                let has_drop=match explicit_request{
                    Some(id)=>self.has_explicit_recoverable_drop(actor,id),
                    None=>self.has_latest_recoverable_drop(actor),
                } && self.state.guarded_restarts.get(&actor).copied().unwrap_or(0)==0;
                ActionResult{
                    code:if has_drop{Code::Ok}else{Code::Empty},
                    payload:if has_drop{b"Dropped".to_vec()}else{vec![]},
                    gas_charged:gas,vm_gas_charged:0,bandwidth_charged:0,
                    exec_step:None,a_credited:0,
                }
            },
            Action::RestartRoot {..} | Action::RestartRootFor{..} => {
                // The organism must explicitly ask. At most one restart
                // after a real S39 drop whose version differs from TODAY.
                if self.state.guarded_restarts.get(&actor).copied().unwrap_or(0)!=0 {
                    return ActionResult::fail(Code::Quota);
                }
                let recoverable=match explicit_request{
                    Some(id)=>self.has_explicit_recoverable_drop(actor,id),
                    None=>self.has_latest_recoverable_drop(actor),
                };
                if !recoverable{return ActionResult::fail(Code::Denied);}
                let Some(hash)=self.state.initial_root_hash.clone()
                    else{return ActionResult::fail(Code::Denied)};
                let root_key=format!("/self/versions/{hash}");
                let Some(root)=self.state.objects.get(&root_key) else{
                    return ActionResult::fail(Code::InvalidCode);
                };
                if root.owner!=actor {
                    return ActionResult::fail(Code::Denied);
                }
                // Only a content-addressed canonical program may become
                // active. No arbitrary restart-provided code is accepted.
                if program_host::preflight_write(
                    &self.state,actor,"/self/active",hash.as_bytes(),None
                ).is_err(){return ActionResult::fail(Code::InvalidCode);}
                let gas=8;
                if let Err(code)=self.spend_a_gas(gas){return ActionResult::fail(code);}
                // S41 lifetime policy: all per-attempt owned outputs
                // beneath these three audited namespaces expire together.
                // This is a PREDECLARED prefix contract (not autonomous
                // ownership inference), but it removes the brittle
                // five-object hard-coded list. Never touch evidence,
                // code versions, tools, active program or other scratch.
                let ephemeral=self.state.objects.iter()
                    .filter(|(key,obj)|obj.owner==actor &&
                        ["/self/results/","/self/work/","/self/discoveries/"]
                            .iter().any(|prefix|key.starts_with(*prefix)))
                    .map(|(key,_)|key.clone()).collect::<Vec<_>>();
                for key in ephemeral {
                    if let Some(obj)=self.state.objects.remove(&key){
                        let host=self.state.organisms.get_mut(&actor).unwrap();
                        host.storage_used=host.storage_used.saturating_sub(obj.bytes.len() as u64);
                    }
                }
                let active=self.state.objects.get_mut("/self/active").unwrap();
                active.bytes=hash.clone().into_bytes();
                active.version+=1;
                program_host::after_write(&mut self.state,actor,Some(hash.clone()));
                self.state.guarded_restarts.insert(actor,1);
                // Old fence remains until S39 root creates a fresh one
                // using newly verified task evidence.
                ActionResult::ok(hash.into_bytes(),gas,0)
            },
            Action::ArmTaskFence {evidence,..} => {
                // A real kernel-issued receipt can arm the SEND fence,
                // but cannot authorize use in a different task version.
                if evidence.len()>256 {return ActionResult::fail(Code::BadRequest);}
                let current=self.state.objects.get("/public/E001/compose-depth")
                    .map(|obj|hash_bytes(&obj.bytes));
                let gas=3;
                if let Err(code)=self.spend_a_gas(gas){return ActionResult::fail(code);}
                let code=match (self.state.probe_record.as_ref(),current.as_ref()){
                    (None,_)=>Code::NotFound,
                    (Some(record),Some(hash)) if record.receipt==evidence &&
                        record.task_hash==*hash=>Code::Ok,
                    _=>Code::Conflict,
                };
                if code==Code::Ok {
                    self.state.task_fences.insert(actor,current.unwrap());
                }
                ActionResult{code,payload:vec![],gas_charged:gas,
                    vm_gas_charged:0,bandwidth_charged:0,exec_step:None,a_credited:0}
            },
            Action::RenewProbe {evidence,..} => {
                // Research-locked v1 -> v2 transition: changing any other
                // task ID or tampering with an old receipt cannot mint
                // another oracle query. The environment must present the
                // exact v2 task at public object version 1.
                if evidence.len()>256 {
                    return ActionResult::fail(Code::BadRequest);
                }
                let Some(obj)=self.state.objects.get("/public/E001/compose-depth")
                    else{return ActionResult::fail(Code::Denied)};
                let Some(task)=serde_json::from_slice::<TransformTask>(&obj.bytes).ok()
                    else{return ActionResult::fail(Code::Denied)};
                if task.task_id!="s37-changed-rule-v2" || obj.version!=1 ||
                    self.state.protocol_service.is_none() {
                    return ActionResult::fail(Code::Denied);
                }
                if self.state.probe_count!=1 ||
                    self.state.probe_renewed_for.is_some() {
                    return ActionResult::fail(Code::Quota);
                }
                let current_hash=hash_bytes(&obj.bytes);
                let gas=7;
                if let Err(code)=self.spend_a_gas(gas) {
                    return ActionResult::fail(code);
                }
                let code=match self.state.probe_record.as_ref(){
                    None=>Code::NotFound,
                    Some(record) if record.receipt!=evidence ||
                        record.task_hash==current_hash => Code::Conflict,
                    Some(_)=>Code::Ok,
                };
                if code==Code::Ok {
                    self.state.probe_renewed_for=Some(current_hash);
                }
                ActionResult{code,payload:vec![],gas_charged:gas,
                    vm_gas_charged:0,bandwidth_charged:0,exec_step:None,a_credited:0}
            },
            Action::VerifyProbe {evidence,..} => {
                // This cannot mint a new label: only validate a PREVIOUS
                // successful Probe issued by this world. Verify after
                // receiving an untrusted /self/ cache read, every time.
                if evidence.len()>256 {return ActionResult::fail(Code::BadRequest);}
                let Some(task)=self.state.objects.get("/public/E001/compose-depth")
                    else{return ActionResult::fail(Code::NotFound)};
                let current_hash=hash_bytes(&task.bytes);
                // Charge for positive and negative verification, including
                // stale tasks or self-cache forgery (but never return label).
                let gas=2;
                if let Err(code)=self.spend_a_gas(gas){
                    return ActionResult::fail(code);
                }
                let Some(record)=self.state.probe_record.as_ref() else{
                    return ActionResult{code:Code::NotFound,payload:vec![],
                        gas_charged:gas,vm_gas_charged:0,
                        bandwidth_charged:0,exec_step:None,a_credited:0};
                };
                let code=if record.task_hash==current_hash &&
                    record.receipt==evidence {Code::Ok}else{Code::Conflict};
                ActionResult{code,payload:vec![],gas_charged:gas,
                    vm_gas_charged:0,bandwidth_charged:0,exec_step:None,a_credited:0}
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
        if self.state.transform_escrow.is_some() && result.code==Code::Ok {
            if let Some(Action::Write{actor,key,expected_version:None,..})=action.as_ref(){
                result.a_credited=self.settle_transform_write(*actor,key,k);
                if result.a_credited==0{self.decline_transform_write(*actor,key,k);}
            }
        }
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
                    let mut feedback=serde_json::to_vec(&result)
                        .expect("serializable action feedback");
                    // The opt-in field is exactly the sum of the two already
                    // visible charges, never an estimate or a future fee.
                    if self.state.transform_escrow.as_ref().and_then(|e|e.fee_total_feedback)==Some(true){
                        if let Some(total)=result.gas_charged.checked_add(result.vm_gas_charged){
                            assert_eq!(feedback.pop(),Some(b'}'));
                            feedback.extend_from_slice(format!(",\"f\":{total}}}").as_bytes());
                        }
                    }
                    host.pending_feedback=Some(feedback);
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
    fn paid_own_prefix_contract(){
        let mut w=World::new(Config::default());
        let bytes=(0..280).map(|i|(i%256) as u8).collect::<Vec<_>>();
        for (key,value) in [("/self/prefix/probe",bytes.clone()),("/self/prefix/empty",vec![])]{
            assert_eq!(w.tick(Some(Action::Write{actor:ORGANISM_ID,key:key.into(),
                bytes:value,expected_version:None})).code,Code::Ok);
        }
        // Explicit foreign fixture; neither prefix nor full read may disclose it.
        w.state.objects.insert("/self/prefix/foreign".into(),Object{owner:0,version:0,bytes:vec![9]});
        let initial=w.snapshot_json();
        let before=w.state.objects.clone();
        let used=w.state.organisms[&ORGANISM_ID].storage_used;
        let read=|key:&str|Action::Read{actor:ORGANISM_ID,key:key.into()};
        let cases=[
            ("/self/prefix/probe",Code::Ok,6,bytes.clone()),
            ("/sys/self-prefix/33/prefix/probe",Code::Ok,3,
                [280u32.to_le_bytes().as_slice(),&bytes[..33]].concat()),
            ("/sys/self-prefix/256/prefix/probe",Code::Ok,7,
                [280u32.to_le_bytes().as_slice(),&bytes[..256]].concat()),
            ("/sys/self-prefix/33/prefix/empty",Code::Ok,3,0u32.to_le_bytes().to_vec()),
            ("/sys/self-prefix/33/prefix/missing",Code::NotFound,0,vec![]),
            ("/sys/self-prefix/33/prefix/foreign",Code::Denied,0,vec![]),
            ("/sys/self-prefix/033/prefix/probe",Code::BadRequest,0,vec![]),
            ("/sys/self-prefix/0/prefix/probe",Code::BadRequest,0,vec![]),
            ("/sys/self-prefix/257/prefix/probe",Code::BadRequest,0,vec![]),
            ("/sys/self-prefix/33/prefix/../probe",Code::BadRequest,0,vec![]),
            ("/sys/self-prefix/33//probe",Code::BadRequest,0,vec![]),
            ("/sys/self-prefix/33/",Code::BadRequest,0,vec![]),
        ];
        let mut replay=World::from_snapshot(&initial).unwrap();
        for (key,code,fee,payload) in cases{
            let a=w.state.nodes[&NodeId::A].gas_available;
            let result=w.tick(Some(read(key)));
            assert_eq!(result.code,code);assert_eq!(result.gas_charged,fee);
            assert_eq!(result.payload,payload);
            assert_eq!(a-w.state.nodes[&NodeId::A].gas_available,fee);
            assert_eq!(result,replay.tick(Some(read(key))));
        }
        assert_eq!(w.state.objects,before);assert_eq!(w.state.organisms[&ORGANISM_ID].storage_used,used);
        assert_eq!(w.snapshot_json(),replay.snapshot_json());assert_eq!(w.audit,replay.audit);
        // True configured low grant (not a balance mutation); a zero-byte write
        // costs at least one A, leaving insufficient gas for the three-A read.
        let mut low=World::new(Config{a_gas_per_epoch:2,..Config::default()});
        // A foreign/missing object may not be used as a proxy for a gas refusal.
        // Install a real existing own object in the initial fixture, with proper storage accounting.
        low.state.objects.insert("/self/prefix/probe".into(),Object{owner:ORGANISM_ID,version:0,bytes:vec![7]});
        low.state.organisms.get_mut(&ORGANISM_ID).unwrap().storage_used=1;
        let r=low.tick(Some(read("/sys/self-prefix/33/prefix/probe")));
        assert_eq!(r.code,Code::Gas);assert_eq!(r.gas_charged,0);assert!(r.payload.is_empty());
        assert_eq!(low.state.nodes[&NodeId::A].gas_available,2);
        println!("S29_PREFIX_CONTRACT full_B=280 full_A=6 prefix_payload_B=37 prefix_A=3 replay=true");
    }

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
    fn s42_stale_drop_cannot_authorize_recovery_for_newer_inflight_request(){
        // Fixture directly arms a known task hash. S39 independently tests
        // that an organism cannot forge an ArmTaskFence capability.
        let mut w=World::with_protocol(Config{
            message_delay:8,..Config::default()
        },Family::P2,42);
        let original=b"task-v1".to_vec();
        w.state.objects.insert("/public/E001/compose-depth".into(),
            Object{owner:0,version:0,bytes:original.clone()});
        w.state.task_fences.insert(ORGANISM_ID,hash_bytes(&original));
        let send=||Action::Send{actor:ORGANISM_ID,
            destination:NodeId::C,bytes:vec![0x10]};
        let first=w.tick(Some(send()));
        assert_eq!(first.code,Code::Ok);
        let first_id=u64::from_le_bytes(first.payload.as_slice().try_into().unwrap());
        for _ in 0..4{w.tick(None);}
        let second=w.tick(Some(send()));
        assert_eq!(second.code,Code::Ok);
        let second_id=u64::from_le_bytes(second.payload.as_slice().try_into().unwrap());
        assert!(second_id>first_id);
        assert_eq!(w.state.last_guarded_send.get(&ORGANISM_ID),Some(&second_id));

        // World revision occurs after BOTH accepted sends, before either
        // service commitment. The first drop is not the latest request.
        let task=w.state.objects.get_mut("/public/E001/compose-depth").unwrap();
        task.bytes=b"task-v2".to_vec();task.version+=1;
        for _ in 0..12{
            if !w.state.fence_drops.is_empty(){break;}
            w.tick(None);
        }
        assert_eq!(w.state.fence_drops.len(),1);
        assert_eq!(w.state.fence_drops[0].message_id,first_id);
        assert!(w.state.guarded_sends.contains_key(&second_id));
        assert!(!w.has_latest_recoverable_drop(ORGANISM_ID));
        let stale=w.tick(Some(Action::FenceStatus{actor:ORGANISM_ID}));
        assert_eq!(stale.code,Code::Empty);
        assert_eq!(stale.gas_charged,1);
        assert_eq!(w.tick(Some(Action::RestartRoot{
            actor:ORGANISM_ID
        })).code,Code::Denied);

        // Only after the newer request is itself actually dropped may
        // FenceStatus report an eligible cause; no B operation executed.
        for _ in 0..12{
            if w.state.fence_drops.len()==2{break;}
            w.tick(None);
        }
        assert_eq!(w.state.fence_drops.len(),2);
        assert_eq!(w.state.fence_drops[1].message_id,second_id);
        assert!(w.has_latest_recoverable_drop(ORGANISM_ID));
        let restored=World::from_snapshot(&w.snapshot_json()).unwrap();
        assert_eq!(w.state_hash(),restored.state_hash());
        assert_eq!(restored.state.last_guarded_send[&ORGANISM_ID],second_id);
        let fresh=w.tick(Some(Action::FenceStatus{actor:ORGANISM_ID}));
        assert_eq!(fresh.code,Code::Ok);
        assert_eq!(fresh.gas_charged,1);
        assert!(w.state.service_events.is_empty());
        assert!(w.state.guarded_restarts.is_empty());
    }

    #[test]
    fn s42_missing_public_task_never_authorizes_restart(){
        let mut w=World::with_protocol(Config{
            message_delay:2,..Config::default()
        },Family::P2,42);
        let task=b"task-v1".to_vec();
        w.state.objects.insert("/public/E001/compose-depth".into(),
            Object{owner:0,version:0,bytes:task.clone()});
        w.state.task_fences.insert(ORGANISM_ID,hash_bytes(&task));
        let sent=w.tick(Some(Action::Send{
            actor:ORGANISM_ID,destination:NodeId::C,bytes:vec![0x10]
        }));
        assert_eq!(sent.code,Code::Ok);
        let id=u64::from_le_bytes(sent.payload.as_slice().try_into().unwrap());
        // The environment removes the public task before service consumption.
        w.state.objects.remove("/public/E001/compose-depth");
        for _ in 0..12{
            if !w.state.fence_drops.is_empty(){break;}
            w.tick(None);
        }
        assert_eq!(w.state.fence_drops.len(),1);
        assert_eq!(w.state.fence_drops[0].message_id,id);
        assert_eq!(w.state.fence_drops[0].observed_hash,None);
        assert!(!w.has_latest_recoverable_drop(ORGANISM_ID));
        let status=w.tick(Some(Action::FenceStatus{actor:ORGANISM_ID}));
        assert_eq!(status.code,Code::Empty);
        assert_eq!(status.gas_charged,1);
        assert_eq!(w.tick(Some(Action::RestartRoot{
            actor:ORGANISM_ID
        })).code,Code::Denied);
        assert!(w.state.guarded_restarts.is_empty());
        assert!(w.state.service_events.is_empty());
        let reloaded=World::from_snapshot(&w.snapshot_json()).unwrap();
        assert_eq!(w.state_hash(),reloaded.state_hash());
    }


    #[test]
    fn s42b_explicit_u64_receipt_blocks_wrong_id_and_pending_sibling(){
        let mut w=World::with_protocol(Config{
            message_delay:8,..Config::default()
        },Family::P2,42);
        // Real successful SEND ids extend beyond u32::MAX.
        w.state.next_sequence=(u32::MAX as u64)+80;
        let old=b"task-v1".to_vec();
        w.state.objects.insert("/public/E001/compose-depth".into(),
            Object{owner:0,version:0,bytes:old.clone()});
        w.state.task_fences.insert(ORGANISM_ID,hash_bytes(&old));
        let send=||Action::Send{actor:ORGANISM_ID,
            destination:NodeId::C,bytes:vec![0x10]};
        let first=w.tick(Some(send()));
        let id1=u64::from_le_bytes(first.payload.as_slice().try_into().unwrap());
        assert_eq!(first.code,Code::Ok);
        assert!(id1>u32::MAX as u64);
        for _ in 0..4 {w.tick(None);}
        let second=w.tick(Some(send()));
        let id2=u64::from_le_bytes(second.payload.as_slice().try_into().unwrap());
        assert_eq!(second.code,Code::Ok);
        assert!(id2>id1);
        let task=w.state.objects.get_mut("/public/E001/compose-depth").unwrap();
        task.bytes=b"task-v2".to_vec();task.version+=1;
        for _ in 0..12{
            if !w.state.fence_drops.is_empty(){break;}
            w.tick(None);
        }
        assert_eq!(w.state.fence_drops.len(),1);
        assert_eq!(w.state.fence_drops[0].message_id,id1);
        assert!(w.state.guarded_sends.contains_key(&id2));
        // Exact id1 is known to be dropped, but other guarded B request
        // remains in flight; cannot restart and later double-execute B.
        let pending=w.tick(Some(Action::FenceStatusFor{
            actor:ORGANISM_ID,request_id:id1}));
        assert_eq!(pending.code,Code::Empty);
        assert_eq!(pending.gas_charged,1);
        assert_eq!(w.tick(Some(Action::RestartRootFor{
            actor:ORGANISM_ID,request_id:id1})).code,Code::Denied);
        // Invented, truncated, and merely adjacent IDs are not evidence.
        for wrong in [id1 as u32 as u64,id1-1]{
            assert_eq!(w.tick(Some(Action::FenceStatusFor{
                actor:ORGANISM_ID,request_id:wrong})).code,Code::Empty);
        }
        for _ in 0..12{
            if w.state.fence_drops.len()==2{break;}
            w.tick(None);
        }
        assert_eq!(w.state.fence_drops[1].message_id,id2);
        assert!(w.state.guarded_sends.is_empty());
        assert_eq!(w.tick(Some(Action::FenceStatusFor{
            actor:ORGANISM_ID,request_id:id1})).code,Code::Ok);
        assert_eq!(w.tick(Some(Action::FenceStatusFor{
            actor:ORGANISM_ID,request_id:id2})).code,Code::Ok);
        assert_eq!(w.tick(Some(Action::FenceStatusFor{
            actor:ORGANISM_ID,request_id:id1 as u32 as u64})).code,Code::Empty);
        assert!(w.state.service_events.is_empty());
        assert_eq!(w.state_hash(),
            World::from_snapshot(&w.snapshot_json()).unwrap().state_hash());
    }

    #[test]
    fn s43a_service_commit_is_not_reply_delivery_or_retry_permission(){
        use ocee_protocol_sim::{Decoded,FrameKind,Operation};
        let mut w=World::with_protocol(Config{
            message_delay:8,..Config::default()
        },Family::P2,42);
        w.state.next_sequence=(u32::MAX as u64)+10;
        let unissued=(u32::MAX as u64)+9;
        let unknown=w.tick(Some(Action::RequestOutcomeFor{
            actor:ORGANISM_ID,request_id:unissued}));
        assert_eq!(unknown.code,Code::Ok);
        assert_eq!(unknown.gas_charged,2);
        let status:RequestOutcome=serde_json::from_slice(&unknown.payload).unwrap();
        assert_eq!(status.state,RequestState::Unknown);
        let mut body=3u16.to_le_bytes().to_vec();
        for n in [1u32,2,3] {body.extend_from_slice(&n.to_le_bytes());}
        let protocol=&w.state.protocol_service.as_ref().unwrap().protocol;
        let wire=protocol.encode(&Decoded{
            kind:FrameKind::Request,operation:Operation::RemoteEval,
            request_id:7,body
        }).unwrap();
        let sent=w.tick(Some(Action::Send{
            actor:ORGANISM_ID,destination:NodeId::C,bytes:wire.clone()}));
        assert_eq!(sent.code,Code::Ok);
        let id=u64::from_le_bytes(sent.payload.as_slice().try_into().unwrap());
        assert!(id>u32::MAX as u64);
        let pending=w.tick(Some(Action::RequestOutcomeFor{
            actor:ORGANISM_ID,request_id:id}));
        let p:RequestOutcome=serde_json::from_slice(&pending.payload).unwrap();
        assert_eq!(p.state,RequestState::InFlight);
        assert_eq!(p.response_message_id,None);

        for _ in 0..30{
            if !w.state.service_events.is_empty(){break;}
            w.tick(None);
        }
        assert_eq!(w.state.service_events.len(),1);
        assert_eq!(w.state.service_events[0].request_message_id,id);
        assert_eq!(w.state.service_events[0].b_gas_charged,20);
        let consumed=w.tick(Some(Action::RequestOutcomeFor{
            actor:ORGANISM_ID,request_id:id}));
        let state:RequestOutcome=serde_json::from_slice(&consumed.payload).unwrap();
        assert_eq!(state.state,RequestState::CommittedB);
        assert_eq!(state.b_gas_charged,Some(20));
        assert_eq!(state.reply_location,Some(ReplyLocation::InTransit));
        let response_id=state.response_message_id.unwrap();
        let reply_in_wire=w.state.pending_deliveries.iter()
            .find(|m|m.id==response_id).unwrap().clone();
        // Even knowing the in-flight response bytes is NOT evidence of
        // receipt. A confirmed reply needs a real audited RECEIVE.
        let early=w.tick(Some(Action::ConfirmResponseFor{
            actor:ORGANISM_ID,request_id:id,
            reply:serde_json::to_vec(&reply_in_wire).unwrap()}));
        assert_eq!(early.code,Code::Conflict);
        assert!(early.gas_charged>=2);

        for _ in 0..30{
            if w.state.mailboxes.get(&NodeId::A)
                .is_some_and(|m|m.iter().any(|r|r.id==response_id)){break;}
            w.tick(None);
        }
        assert!(w.state.mailboxes.get(&NodeId::A)
            .is_some_and(|m|m.iter().any(|r|r.id==response_id)));
        let received=w.tick(Some(Action::Receive{
            actor:ORGANISM_ID,max_count:1}));
        assert_eq!(received.code,Code::Ok);
        let replies:Vec<Message>=serde_json::from_slice(&received.payload).unwrap();
        assert_eq!(replies.len(),1);
        assert_eq!(replies[0].id,response_id);
        let raw=serde_json::to_vec(&replies[0]).unwrap();
        let validated=w.tick(Some(Action::ConfirmResponseFor{
            actor:ORGANISM_ID,request_id:id,reply:raw.clone()}));
        assert_eq!(validated.code,Code::Ok);
        let proof:ConfirmedReply=serde_json::from_slice(&validated.payload).unwrap();
        assert_eq!(proof.request_id,id);
        assert_eq!(proof.response_message_id,response_id);
        assert_eq!(proof.b_gas_charged,20);
        let mut tampered=replies[0].clone();
        tampered.bytes.push(0xaa);
        // Stronger than a never-observed forged reply: the counterfeit
        // genuinely passes through RECEIVE, yet must not be attributed
        // to the service's event with the same response message ID.
        w.state.mailboxes.entry(NodeId::A).or_default().push(tampered.clone());
        let forged_receive=w.tick(Some(Action::Receive{
            actor:ORGANISM_ID,max_count:1}));
        assert_eq!(forged_receive.code,Code::Ok);
        let observed:Vec<Message>=serde_json::from_slice(
            &forged_receive.payload).unwrap();
        assert_eq!(observed,vec![tampered.clone()]);
        assert_eq!(w.tick(Some(Action::ConfirmResponseFor{
            actor:ORGANISM_ID,request_id:id,
            reply:serde_json::to_vec(&tampered).unwrap()})).code,Code::Conflict);
        // A legitimate already-received reply remains valid despite
        // an adversarial duplicate being present in the audit log.
        assert_eq!(w.tick(Some(Action::ConfirmResponseFor{
            actor:ORGANISM_ID,request_id:id,
            reply:serde_json::to_vec(&replies[0]).unwrap()})).code,Code::Ok);
        assert_eq!(w.tick(Some(Action::ConfirmResponseFor{
            actor:ORGANISM_ID,request_id:id+999,reply:raw.clone()})).code,Code::Conflict);

        // The same protocol-level u32 job ID is submitted AGAIN with a new
        // kernel-level u64 message ID. The service deduplicates B work;
        // the first reply must NOT confirm the second transport request.
        let duplicate=w.tick(Some(Action::Send{
            actor:ORGANISM_ID,destination:NodeId::C,bytes:wire}));
        assert_eq!(duplicate.code,Code::Ok);
        let second_id=u64::from_le_bytes(
            duplicate.payload.as_slice().try_into().unwrap());
        assert_ne!(second_id,id);
        for _ in 0..30{
            if w.state.service_events.len()==2{break;}
            w.tick(None);
        }
        assert_eq!(w.state.service_events.len(),2);
        let repeated=w.tick(Some(Action::RequestOutcomeFor{
            actor:ORGANISM_ID,request_id:second_id}));
        let repeated:RequestOutcome=serde_json::from_slice(&repeated.payload).unwrap();
        assert_eq!(repeated.state,RequestState::HandledNoB);
        assert_eq!(repeated.b_gas_charged,Some(0));
        assert_ne!(repeated.response_message_id,state.response_message_id);
        assert_eq!(w.tick(Some(Action::ConfirmResponseFor{
            actor:ORGANISM_ID,request_id:second_id,reply:raw}))
            .code,Code::Conflict);
        let after=w.tick(Some(Action::RequestOutcomeFor{
            actor:ORGANISM_ID,request_id:id}));
        let after:RequestOutcome=serde_json::from_slice(&after.payload).unwrap();
        assert_eq!(after.state,RequestState::CommittedB);
        assert_eq!(after.reply_location,Some(ReplyLocation::NotQueued));
        assert_eq!(w.state.service_events.iter()
            .map(|e|e.b_gas_charged).sum::<u64>(),20);
        assert_eq!(w.state_hash(),
            World::from_snapshot(&w.snapshot_json()).unwrap().state_hash());
    }

    #[test]
    fn s43a_drop_and_handled_without_b_are_not_unknown_or_success(){
        let mut dropped=World::with_protocol(Config{
            message_delay:2,..Config::default()
        },Family::P2,42);
        let task=b"task-v1".to_vec();
        dropped.state.objects.insert("/public/E001/compose-depth".into(),
            Object{owner:0,version:0,bytes:task.clone()});
        dropped.state.task_fences.insert(ORGANISM_ID,hash_bytes(&task));
        let sent=dropped.tick(Some(Action::Send{
            actor:ORGANISM_ID,destination:NodeId::C,bytes:vec![0x10]}));
        let id=u64::from_le_bytes(sent.payload.as_slice().try_into().unwrap());
        let t=dropped.state.objects.get_mut("/public/E001/compose-depth").unwrap();
        t.bytes=b"task-v2".to_vec();t.version+=1;
        for _ in 0..12{
            if !dropped.state.fence_drops.is_empty(){break;}
            dropped.tick(None);
        }
        assert_eq!(dropped.state.fence_drops.len(),1);
        let state=dropped.tick(Some(Action::RequestOutcomeFor{
            actor:ORGANISM_ID,request_id:id}));
        let o:RequestOutcome=serde_json::from_slice(&state.payload).unwrap();
        assert_eq!(o.state,RequestState::Dropped);
        assert_eq!(o.b_gas_charged,None);
        assert!(dropped.state.service_events.is_empty());

        let mut no_b=World::with_protocol(Config{
            message_delay:2,..Config::default()
        },Family::P2,42);
        let malformed=no_b.tick(Some(Action::Send{
            actor:ORGANISM_ID,destination:NodeId::C,bytes:vec![0x10]}));
        let bad_id=u64::from_le_bytes(malformed.payload.as_slice().try_into().unwrap());
        for _ in 0..12{
            if !no_b.state.service_events.is_empty(){break;}
            no_b.tick(None);
        }
        assert_eq!(no_b.state.service_events.len(),1);
        assert_eq!(no_b.state.service_events[0].b_gas_charged,0);
        assert!(no_b.state.service_events[0].diagnostic.is_some());
        let q=no_b.tick(Some(Action::RequestOutcomeFor{
            actor:ORGANISM_ID,request_id:bad_id}));
        let o:RequestOutcome=serde_json::from_slice(&q.payload).unwrap();
        assert_eq!(o.state,RequestState::HandledNoB);
        assert_eq!(o.b_gas_charged,Some(0));
        assert_eq!(no_b.state_hash(),
            World::from_snapshot(&no_b.snapshot_json()).unwrap().state_hash());
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
