use soroban_sdk::{Env, Address, panic};

#[derive(Clone)]
pub struct ExpertJuror {
    pub juror: Address,
    pub domain_tags: Vec<String>, // e.g. ["Frontend", "Rust", "Marketing"]
}

pub fn add_juror(env: &Env, juror: Address, tags: Vec<String>) {
    require_governance(env);
    env.storage().set(&format!("juror:{}", juror), &ExpertJuror {
        juror: juror.clone(),
        domain_tags: tags,
    });
}

pub fn remove_juror(env: &Env, juror: Address) {
    require_governance(env);
    env.storage().remove(&format!("juror:{}", juror));
}

#[derive(Clone)]
pub struct Dispute {
    pub dispute_id: String,
    pub domain_tag: String,
    pub initiator: Address,
    pub timestamp: u64,
}

pub fn trigger_dispute(env: &Env, dispute_id: String, domain_tag: String) {
    let initiator = env.invoker();
    let dispute = Dispute {
        dispute_id: dispute_id.clone(),
        domain_tag: domain_tag.clone(),
        initiator,
        timestamp: env.ledger().timestamp(),
    };
    env.storage().set(&format!("dispute:{}", dispute_id), &dispute);

    env.events().publish(
        (["dispute", "triggered"],),
        (dispute_id, domain_tag),
    );
}

pub fn get_eligible_jurors(env: &Env, domain_tag: String) -> Vec<Address> {
    let mut eligible: Vec<Address> = Vec::new(env);

    // Iterate through all jurors (assuming registry stored separately)
    let jurors: Option<Vec<Address>> = env.storage().get("jurors");
    if let Some(list) = jurors {
        for j in list.iter() {
            let record: Option<ExpertJuror> = env.storage().get(&format!("juror:{}", j));
            if let Some(r) = record {
                if r.domain_tags.contains(&domain_tag) {
                    eligible.push(j.clone());
                }
            }
        }
    }
    eligible
}

fn require_governance(env: &Env) {
    let caller = env.invoker();
    let governor: Address = env.storage().get("governor").unwrap();
    if caller != governor {
        panic!("Only governance can call this function");
    }
}
