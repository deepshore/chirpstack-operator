use chirpstack_operator::crd::Chirpstack;
use kube::CustomResourceExt;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let crd = serde_json::to_string(&Chirpstack::crd())?;
    println!("{crd}");

    Ok(())
}
