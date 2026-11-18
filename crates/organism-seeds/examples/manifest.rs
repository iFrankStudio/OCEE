fn main() {
    let manifests=[
        serde_json::to_value(ocee_organism_seeds::s0_manifest()).unwrap(),
        serde_json::to_value(ocee_organism_seeds::p0_probe::s1_manifest()).unwrap(),
        serde_json::to_value(ocee_organism_seeds::s2_p0::s2_manifest()).unwrap(),
        serde_json::to_value(ocee_organism_seeds::p0_probe::s1_tool_manifest()).unwrap(),
        serde_json::to_value(ocee_organism_seeds::s4_structural::s4_manifest()).unwrap(),
        serde_json::to_value(ocee_organism_seeds::s5_search::s5_manifest()).unwrap(),
        serde_json::to_value(ocee_organism_seeds::s6_compose::s6_manifest()).unwrap(),
        serde_json::to_value(ocee_organism_seeds::s7_recover::s7_manifest()).unwrap(),
        serde_json::to_value(ocee_organism_seeds::s8_repair_search::s8_manifest()).unwrap(),
        serde_json::to_value(ocee_organism_seeds::s9_witness::s9_manifest()).unwrap(),
        serde_json::to_value(ocee_organism_seeds::s10_semantic::s10_manifest()).unwrap(),
        serde_json::to_value(ocee_organism_seeds::s11_audit::s11_manifest()).unwrap(),
        serde_json::to_value(ocee_organism_seeds::s12_provenance::s12_manifest()).unwrap(),
    ];
    println!("{}",serde_json::to_string_pretty(&manifests).unwrap());
}
