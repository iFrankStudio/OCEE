fn main() {
    let manifests=[
        serde_json::to_value(ocee_organism_seeds::s0_manifest()).unwrap(),
        serde_json::to_value(ocee_organism_seeds::p0_probe::s1_manifest()).unwrap(),
        serde_json::to_value(ocee_organism_seeds::s2_p0::s2_manifest()).unwrap(),
        serde_json::to_value(ocee_organism_seeds::p0_probe::s1_tool_manifest()).unwrap(),
        serde_json::to_value(ocee_organism_seeds::s4_structural::s4_manifest()).unwrap(),
    ];
    println!("{}",serde_json::to_string_pretty(&manifests).unwrap());
}
