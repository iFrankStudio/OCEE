//! S21A: a metered ACTIVE organism infers P2 TLV field roles and physical
//! order from the PUBLIC request/response HELLO pair; the VM receives no
//! private tags, field ordering, protocol seed or remote opcode.
//!
//! Researcher priors: 1B marker + 2B BE envelope + single outer TLV; five
//! inner TLVs; 1B kind 0->1, same 1B HELLO opcode, 4B ID, request empty
//! body/response nonempty body, one XOR check TLV. Five-way classifier,
//! candidate tests and success policy are researcher-written graph logic.
//! This is not from-scratch unknown grammar invention nor E001 evidence.
#![forbid(unsafe_code)]
use ocee_organism_vm::{
    Block,Function,Instruction,Op,Param,Program,Terminator,Type,WorldCall
};
fn lit(data:&[u8])->Op{
    Op::ConstBytes{hex:data.iter().map(|b|format!("{b:02x}")).collect()}
}
fn instr(id:u32,op:Op)->Instruction{Instruction{id,op}}
fn param(id:u32,ty:Type)->Param{Param{id,ty}}
fn blk(name:&str,params:Vec<Param>,instructions:Vec<Instruction>,
    terminator:Terminator)->Block{
    Block{id:name.into(),params,instructions,terminator}
}
fn j(name:&str,args:Vec<u32>)->Terminator{
    Terminator::Jump{target:name.into(),args}
}
fn br(c:u32,a:&str,aa:Vec<u32>,b:&str,bb:Vec<u32>)->Terminator{
    Terminator::Branch{condition:c,if_true:a.into(),true_args:aa,
        if_false:b.into(),false_args:bb}
}
fn y(call:WorldCall,args:Vec<u32>,next:&str,carry:Vec<u32>)->Terminator{
    Terminator::Yield{call,args,next:next.into(),next_args:carry}
}
// State registers 1000..1010 in every stage:
// req_fields, response_fields, req_inner, response_inner (Bytes);
// observed_marker, kind_tag, opcode_tag, id_tag, body_tag, check_tag (U32);
// original physical field order (Bytes).
fn state()->Vec<Param>{
    (0..11).map(|k|param(1000+k,if k<4 || k==10{
        Type::Bytes
    }else{Type::U32})).collect()
}
fn reg()->Vec<u32>{(1000..1011).collect()}
fn candidate()->Vec<Param>{
    let mut v=state();
    v.extend([
        param(1011,Type::U32), // current observed field tag
        param(1012,Type::Bytes), // request value
        param(1013,Type::Bytes), // response value
    ]);
    v
}
fn cand()->Vec<u32>{(1000..1014).collect()}
fn next(i:usize)->String{
    if i==4 {"verify_checksum".into()}else{format!("stage{}",i+1)}
}
fn classify_stage(index:usize)->Vec<Block>{
    let prefix=format!("stage{index}");
    let mut blocks=Vec::new();
    let normal=reg();
    let full=cand();
    let mut read=vec![
        instr(1100,Op::ConstU32(index as u32)),
        instr(1101,Op::JsonIndex(1000,1100)),
        instr(1102,Op::JsonIndex(1001,1100)),
        instr(1103,lit(b"tag")),
        instr(1104,Op::JsonFieldU32(1101,1103)),
        instr(1105,Op::JsonFieldU32(1102,1103)),
        instr(1106,lit(b"value")),
        instr(1107,Op::JsonFieldBytes(1101,1106)),
        instr(1108,Op::JsonFieldBytes(1102,1106)),
        instr(1109,Op::EqU32(1104,1105)),
    ];
    let mut with=normal.clone();
    with.extend([1104,1107,1108]);
    blocks.push(blk(&prefix,state(),std::mem::take(&mut read),
        br(1109,&format!("{prefix}_id"),with,"failed",vec![])));
    blocks.push(blk(&format!("{prefix}_id"),candidate(),vec![
        instr(1110,Op::Len(1012)),
        instr(1111,Op::ConstU32(4)),
        instr(1112,Op::EqU32(1110,1111)),
    ],br(1112,&format!("{prefix}_store_id"),full.clone(),
        &format!("{prefix}_body"),full.clone())));
    blocks.push(blk(&format!("{prefix}_body"),candidate(),vec![
        instr(1113,Op::Len(1012)),
        instr(1114,Op::ConstU32(0)),
        instr(1115,Op::EqU32(1113,1114)),
    ],br(1115,&format!("{prefix}_store_body"),full.clone(),
        &format!("{prefix}_onebyte"),full.clone())));
    blocks.push(blk(&format!("{prefix}_onebyte"),candidate(),vec![
        instr(1116,Op::Len(1012)),
        instr(1117,Op::ConstU32(1)),
        instr(1118,Op::EqU32(1116,1117)),
    ],br(1118,&format!("{prefix}_kind_request"),full.clone(),
        "failed",vec![])));
    blocks.push(blk(&format!("{prefix}_kind_request"),candidate(),vec![
        instr(1119,lit(&[0])),
        instr(1120,Op::EqBytes(1012,1119)),
    ],br(1120,&format!("{prefix}_kind_response"),full.clone(),
        &format!("{prefix}_opcode"),full.clone())));
    blocks.push(blk(&format!("{prefix}_kind_response"),candidate(),vec![
        instr(1121,lit(&[1])),
        instr(1122,Op::EqBytes(1013,1121)),
    ],br(1122,&format!("{prefix}_store_kind"),full.clone(),
        &format!("{prefix}_opcode"),full.clone())));
    blocks.push(blk(&format!("{prefix}_opcode"),candidate(),vec![
        instr(1123,Op::EqBytes(1012,1013)),
    ],br(1123,&format!("{prefix}_store_opcode"),full.clone(),
        &format!("{prefix}_store_check"),full.clone())));
    for (name,slot) in [("id",7usize),("body",8),("kind",5),("opcode",6),("check",9)]{
        let mut carry=normal.clone();
        carry[slot]=1011;
        // Capture actual physical order from public request, never sort tags.
        blocks.push(blk(&format!("{prefix}_store_{name}"),candidate(),vec![
            instr(1130,Op::PackByte(1011)),
            instr(1131,Op::Concat(1010,1130)),
        ],{
            carry[10]=1131;
            j(&next(index),carry)
        }));
    }
    blocks
}
fn build(no_response:bool)->Program{
    let b=Type::Bytes;
    let u=Type::U32;
    let mut blocks=vec![
        blk("entry",vec![param(0,b)],vec![
            instr(1,lit(b"/public/C/hello")),
        ],y(WorldCall::Read,vec![1],"sample",vec![])),
        blk("sample",vec![param(10,b)],vec![
            instr(11,lit(b"payload")),
            instr(12,Op::JsonFieldBytes(10,11)),
            instr(13,lit(b"request")),
            instr(14,lit(if no_response{b"request"}else{b"response"})),
            instr(15,Op::JsonFieldBytes(12,13)),
            instr(16,Op::JsonFieldBytes(12,14)),
            instr(17,Op::ConstU32(0)),
            instr(18,Op::ByteAt(15,17)),
            instr(19,Op::ByteAt(16,17)),
            instr(20,Op::EqU32(18,19)),
        ],br(20,"outer",vec![15,16,18],"failed",vec![])),
        blk("outer",vec![param(21,b),param(22,b),param(23,u)],vec![
            instr(24,Op::ConstU32(3)),
            instr(25,Op::Len(21)),
            instr(26,Op::Len(22)),
            instr(27,Op::Sub(25,24)),
            instr(28,Op::Sub(26,24)),
            instr(29,Op::Slice(21,24,27)),
            instr(30,Op::Slice(22,24,28)),
            instr(31,Op::TlvDecode(29)),
            instr(32,Op::TlvDecode(30)),
            instr(33,Op::ConstU32(0)),
            instr(34,Op::JsonIndex(31,33)),
            instr(35,Op::JsonIndex(32,33)),
            instr(36,lit(b"tag")),
            instr(37,Op::JsonFieldU32(34,36)),
            instr(38,Op::JsonFieldU32(35,36)),
            instr(39,Op::ConstU32(0x10)),
            instr(40,Op::EqU32(37,39)),
            instr(41,Op::EqU32(38,39)),
        ],br(40,"outer_response",vec![34,35,23,41],
            "failed",vec![])),
        blk("outer_response",vec![
            param(50,b),param(51,b),param(52,u),param(53,Type::Bool)
        ],vec![
            instr(54,lit(b"value")),
            instr(55,Op::JsonFieldBytes(50,54)),
            instr(56,Op::JsonFieldBytes(51,54)),
            instr(57,Op::TlvDecode(55)),
            instr(58,Op::TlvDecode(56)),
            instr(59,Op::ConstU32(u32::MAX)),
            instr(60,lit(b"")),
        ],br(53,"stage0",
            vec![57,58,55,56,52,59,59,59,59,59,60],
            "failed",vec![])),
    ];
    for n in 0..5 {blocks.extend(classify_stage(n));}
    // Actual XOR consistency of TLV wire, not host-provided validator:
    // checksum TLV contributes tag + BE len(1) + checksum byte, so
    // XOR(all physical TLV bytes) == observed check_tag XOR 1.
    blocks.extend([
        blk("verify_checksum",state(),vec![
            instr(1200,Op::XorFold(1002)),
            instr(1201,Op::XorFold(1003)),
            instr(1202,Op::ConstU32(1)),
            instr(1203,Op::Xor(1009,1202)),
            instr(1204,Op::EqU32(1200,1203)),
            instr(1205,Op::EqU32(1201,1203)),
        ],br(1204,"verify_response",reg(),"failed",vec![])),
        blk("verify_response",state(),vec![
            instr(1210,Op::XorFold(1003)),
            instr(1211,Op::ConstU32(1)),
            instr(1212,Op::Xor(1009,1211)),
            instr(1213,Op::EqU32(1210,1212)),
        ],br(1213,"save_roles",reg(),"failed",vec![])),
        // Stored 11B structure: observed P2 marker, role-tag tuple
        // [kind,opcode,id,body,check], then physical ordered five tags.
        // This is evidence, not a host-provided P2 grammar label.
        blk("save_roles",state(),vec![
            instr(1220,Op::PackByte(1004)),
            instr(1221,Op::PackByte(1005)),
            instr(1222,Op::Concat(1220,1221)),
            instr(1223,Op::PackByte(1006)),
            instr(1224,Op::Concat(1222,1223)),
            instr(1225,Op::PackByte(1007)),
            instr(1226,Op::Concat(1224,1225)),
            instr(1227,Op::PackByte(1008)),
            instr(1228,Op::Concat(1226,1227)),
            instr(1229,Op::PackByte(1009)),
            instr(1230,Op::Concat(1228,1229)),
            instr(1231,Op::Concat(1230,1010)),
            instr(1232,lit(b"/self/discoveries/p2-tlv-roles")),
            instr(1233,Op::ConstU32(u32::MAX)),
        ],y(WorldCall::Write,vec![1232,1231,1233],"success",vec![])),
        blk("success",vec![param(1300,b)],vec![
            instr(1301,Op::ConstU32(1)),
        ],Terminator::Return{value:1301}),
        blk("failed",vec![],vec![
            instr(1302,Op::ConstU32(0)),
        ],Terminator::Return{value:1302}),
    ]);
    let program=Program{
        format_version:1,entry_function:"main".into(),
        functions:vec![Function{
            name:"main".into(),args:vec![b],returns:u,blocks
        }]
    };
    program.validate().expect("S21 paired public HELLO grammar classifier valid");
    program
}
pub fn s21_p2_infer_public_hello_roles()->Program{build(false)}
/// If the response half is unavailable, the same kind-change evidence must
/// be absent. This is a controlled *researcher-side* ablation, not VM learning.
pub fn s21_p2_request_only_ablation()->Program{build(true)}

pub fn s21_manifest()->crate::SeedManifest{
    crate::SeedManifest{
        manifest_version:1,
        kind:"S21A-active-P2-public-HELLO-role-inference",
        program_hash:format!("sha256:{}",s21_p2_infer_public_hello_roles()
            .content_hash().unwrap()),
        built_in_modules:vec![
            "generic-gas-metered-TlvDecode-and-TlvEncode-primitives",
            "researcher-seeded-5-field-paired-TLV-role-classification",
            "derive-marker-tags-and-physical-order-only-from-public-HELLO",
            "role-inference-by-length-kind-change-and-unchanged-HELLO-opcode",
            "verify-xor-over-physical-request-and-response-inner-TLV",
            "persist-11-byte-observed-structure-under-self-discoveries",
        ],
        initial_tool_hashes:vec![],
        known_protocol_families:vec!["hypothesis-single-outer-TLV-five-fields".into()],
        prohibited_knowledge:vec![
            "private-P2-tags","private-P2-permutation","protocol-seed",
            "hidden-REMOTE_EVAL-opcode","hidden-RESULT-opcode",
            "B-computation-gas","future-work-item-answer",
        ],
        research_status:"DEVELOPMENT_VM_P2_PAIRED_HELLO_ROLE_INFERENCE_NOT_REMOTE_WORK_NOT_E001",
    }
}


/// S21B: after discovering P2 roles, the SAME active VM composes a non-replay
/// HELLO request with request_id=73, using ONLY tags/order and HELLO opcode
/// observed in public data; sends it to real C and saves a checked reply.
/// Unlike E001, this uses a researcher-known P2 TLV envelope hypothesis and
/// already-known HELLO semantics. It does not search REMOTE_EVAL/RESULT yet.
pub fn s21_p2_infer_and_validate_new_hello()->Program {
    let mut program=build(false);
    let f=&mut program.functions[0];
    let b=Type::Bytes;
    let u=Type::U32;
    let save=f.blocks.iter_mut().find(|node|node.id=="save_roles").unwrap();
    save.terminator=y(WorldCall::Write,vec![1232,1231,1233],
        "probe_read",vec![1231]);
    f.blocks.extend([
        blk("probe_read",vec![param(2000,b),param(2001,b)],vec![
            instr(2002,lit(b"/public/C/hello")),
        ],y(WorldCall::Read,vec![2002],"probe_sample",vec![2000])),
        blk("probe_sample",vec![param(2010,b),param(2011,b)],vec![
            instr(2012,lit(b"payload")),
            instr(2013,Op::JsonFieldBytes(2011,2012)),
            instr(2014,lit(b"request")),
            instr(2015,Op::JsonFieldBytes(2013,2014)),
            instr(2016,Op::Len(2015)),
            instr(2017,Op::ConstU32(6)),
            instr(2018,Op::Sub(2016,2017)),
            instr(2019,Op::Slice(2015,2017,2018)), // known P2 outer TLV structure
            instr(2020,Op::ConstU32(0)),
            instr(2021,Op::ConstU32(1)),
            instr(2022,Op::ConstU32(2)),
            instr(2023,Op::ConstU32(3)),
            instr(2024,Op::ConstU32(4)),
            instr(2025,Op::ConstU32(5)),
            instr(2026,Op::ConstU32(6)),
            instr(2027,Op::ByteAt(2010,2020)), // observed marker
            instr(2028,Op::ByteAt(2010,2021)), // inferred kind tag
            instr(2029,Op::ByteAt(2010,2022)), // inferred opcode tag
            instr(2030,Op::ByteAt(2010,2023)), // inferred id tag
            instr(2031,Op::ByteAt(2010,2024)), // inferred body tag
            instr(2032,Op::ByteAt(2010,2025)), // inferred checksum tag
            instr(2033,Op::Slice(2010,2026,2025)), // 5B original physical order
            instr(2034,Op::TlvLookup(2019,2029)), // HELLO opcode value from public sample
            instr(2035,lit(&[0])),
            instr(2036,Op::TlvEncode(2028,2035)),
            instr(2037,Op::TlvEncode(2029,2034)),
            instr(2038,lit(&[73,0,0,0])),
            instr(2039,Op::TlvEncode(2030,2038)),
            instr(2040,lit(b"")),
            instr(2041,Op::TlvEncode(2031,2040)),
            instr(2042,Op::Concat(2036,2037)),
            instr(2043,Op::Concat(2042,2039)),
            instr(2044,Op::Concat(2043,2041)),
            instr(2045,Op::XorFold(2044)),
            instr(2046,Op::PackByte(2045)),
            instr(2047,Op::TlvEncode(2032,2046)),
            instr(2048,Op::Concat(2044,2047)),
            instr(2049,Op::TlvReorder(2048,2033)), // physical order inferred by VM
            instr(2050,Op::ConstU32(0x10)), // researcher-seeded outer TLV hypothesis
            instr(2051,Op::TlvEncode(2050,2049)),
            instr(2052,Op::Len(2051)),
            instr(2053,Op::PackU16Be(2052)),
            instr(2054,Op::PackByte(2027)),
            instr(2055,Op::Concat(2054,2053)),
            instr(2056,Op::Concat(2055,2051)),
        ],y(WorldCall::Send,vec![2056],"probe_wait1",vec![2010,2034])),
        blk("probe_wait1",vec![param(2060,b),param(2061,b),param(2062,b)],
            vec![],y(WorldCall::Observe,vec![],"probe_wait2",vec![2060,2061])),
        blk("probe_wait2",vec![param(2070,b),param(2071,b),param(2072,b)],
            vec![],y(WorldCall::Observe,vec![],"probe_wait3",vec![2070,2071])),
        blk("probe_wait3",vec![param(2080,b),param(2081,b),param(2082,b)],
            vec![],y(WorldCall::Observe,vec![],"probe_recv",vec![2080,2081])),
        blk("probe_recv",vec![param(2090,b),param(2091,b),param(2092,b)],
            vec![],y(WorldCall::Receive,vec![],"probe_reply",vec![2090,2091])),
        blk("probe_reply",vec![param(2100,b),param(2101,b),param(2102,b)],vec![
            instr(2103,lit(b"payload")),
            instr(2104,Op::JsonFieldBytes(2102,2103)),
            instr(2105,Op::ConstU32(0)),
            instr(2106,Op::JsonIndex(2104,2105)),
            instr(2107,lit(b"bytes")),
            instr(2108,Op::JsonFieldBytes(2106,2107)),
            instr(2109,Op::ByteAt(2108,2105)),
            instr(2110,Op::ByteAt(2100,2105)),
            instr(2111,Op::EqU32(2109,2110)),
        ],br(2111,"probe_outer",vec![2100,2101,2108],
            "failed",vec![])),
        blk("probe_outer",vec![
            param(2120,b),param(2121,b),param(2122,b)
        ],vec![
            instr(2123,Op::ConstU32(3)),
            instr(2124,Op::Len(2122)),
            instr(2125,Op::Sub(2124,2123)),
            instr(2126,Op::Slice(2122,2123,2125)),
            instr(2127,Op::PackU16Be(2125)),
            instr(2128,Op::ConstU32(1)),
            instr(2129,Op::ConstU32(2)),
            instr(2130,Op::Slice(2122,2128,2129)),
            instr(2131,Op::EqBytes(2127,2130)),
        ],br(2131,"probe_tlv_outer",vec![2120,2121,2122,2126],
            "failed",vec![])),
        blk("probe_tlv_outer",vec![
            param(2140,b),param(2141,b),param(2142,b),param(2143,b)
        ],vec![
            instr(2144,Op::TlvDecode(2143)),
            instr(2145,Op::ConstU32(0)),
            instr(2146,Op::JsonIndex(2144,2145)),
            instr(2147,lit(b"tag")),
            instr(2148,Op::JsonFieldU32(2146,2147)),
            instr(2149,Op::ConstU32(0x10)),
            instr(2150,Op::EqU32(2148,2149)),
        ],br(2150,"probe_kind",vec![2140,2141,2142,2146],
            "failed",vec![])),
        blk("probe_kind",vec![
            param(2160,b),param(2161,b),param(2162,b),param(2163,b)
        ],vec![
            instr(2164,lit(b"value")),
            instr(2165,Op::JsonFieldBytes(2163,2164)),
            instr(2166,Op::ConstU32(1)),
            instr(2167,Op::ByteAt(2160,2166)),
            instr(2168,Op::TlvLookup(2165,2167)),
            instr(2169,lit(&[1])),
            instr(2170,Op::EqBytes(2168,2169)),
        ],br(2170,"probe_opcode",vec![2160,2161,2162,2165],
            "failed",vec![])),
        blk("probe_opcode",vec![
            param(2180,b),param(2181,b),param(2182,b),param(2183,b)
        ],vec![
            instr(2184,Op::ConstU32(2)),
            instr(2185,Op::ByteAt(2180,2184)),
            instr(2186,Op::TlvLookup(2183,2185)),
            instr(2187,Op::EqBytes(2186,2181)),
        ],br(2187,"probe_id",vec![2180,2181,2182,2183],
            "failed",vec![])),
        blk("probe_id",vec![
            param(2190,b),param(2191,b),param(2192,b),param(2193,b)
        ],vec![
            instr(2194,Op::ConstU32(3)),
            instr(2195,Op::ByteAt(2190,2194)),
            instr(2196,Op::TlvLookup(2193,2195)),
            instr(2197,lit(&[73,0,0,0])),
            instr(2198,Op::EqBytes(2196,2197)),
        ],br(2198,"probe_body",vec![2190,2191,2192,2193],
            "failed",vec![])),
        blk("probe_body",vec![
            param(2200,b),param(2201,b),param(2202,b),param(2203,b)
        ],vec![
            instr(2204,Op::ConstU32(4)),
            instr(2205,Op::ByteAt(2200,2204)),
            instr(2206,Op::TlvLookup(2203,2205)),
            instr(2207,lit(&[0,1])), // seeded HELLO response status/version
            instr(2208,Op::EqBytes(2206,2207)),
        ],br(2208,"probe_checksum",vec![2200,2202,2203],
            "failed",vec![])),
        blk("probe_checksum",vec![
            param(2210,b),param(2211,b),param(2212,b)
        ],vec![
            instr(2213,Op::ConstU32(5)),
            instr(2214,Op::ByteAt(2210,2213)),
            instr(2215,Op::XorFold(2212)),
            instr(2216,Op::ConstU32(1)),
            instr(2217,Op::Xor(2214,2216)),
            instr(2218,Op::EqU32(2215,2217)),
        ],br(2218,"probe_save",vec![2211],"failed",vec![])),
        blk("probe_save",vec![param(2220,b)],vec![
            instr(2221,lit(b"/self/observations/p2-hello-nonreplay")),
            instr(2222,Op::ConstU32(u32::MAX)),
        ],y(WorldCall::Write,vec![2221,2220,2222],"success",vec![])),
    ]);
    program.validate().expect("S21B observed-role new HELLO VM must typecheck");
    program
}
pub fn s21_p2_new_hello_no_live_check_ablation()->Program {
    let mut p=s21_p2_infer_and_validate_new_hello();
    let verify=p.functions[0].blocks.iter_mut()
        .find(|b|b.id=="probe_body").unwrap();
    verify.instructions.iter_mut().find(|i|i.id==2207).unwrap().op=
        lit(&[0,7]); // wrong observed response; cannot accept a real C reply
    p.validate().expect("S21B wrong-reply control must remain well-typed");
    p
}
pub fn s21b_manifest()->crate::SeedManifest {
    let program=s21_p2_infer_and_validate_new_hello();
    crate::SeedManifest{
        manifest_version:1,
        kind:"S21B-active-P2-inferred-roles-valid-new-HELLO",
        program_hash:format!("sha256:{}",program.content_hash().unwrap()),
        built_in_modules:vec![
            "S21A-metered-public-P2-role-and-physical-order-inference",
            "generic-TlvLookup-TlvEncode-TlvReorder",
            "construct-nonreplayed-request-id73-HELLO-with-observed-opcode",
            "live-A-to-C-SEND-OBSERVE-RECEIVE",
            "verify-response-marker-envelope-kind-op-id-body-and-xor",
            "persist-only-verified-new-C-response",
        ],
        initial_tool_hashes:vec![],
        known_protocol_families:vec!["researcher-hypothesis-five-TLV-fields".into()],
        prohibited_knowledge:vec![
            "private-P2-role-tags","private-P2-tag-order","protocol-seed",
            "private-REMOTE_EVAL-opcode","private-RESULT-opcode",
            "B-gas-ledger","held-out-FOLD32-answer",
        ],
        research_status:"DEVELOPMENT_P2_NEW_HELLO_VM_NO_REMOTE_EVAL_NO_NEW_GRAMMAR_NOT_E001",
    }
}
