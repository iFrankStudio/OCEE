fn main() {
    println!("{}",serde_json::to_string_pretty(&ocee_organism_seeds::s0_manifest()).unwrap());
}
