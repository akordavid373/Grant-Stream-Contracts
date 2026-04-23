use soroban_sdk::{Env, Address, panic};

#[derive(Clone)]
pub struct FeeConfig {
    pub platform_fee_bps: i128, // basis points, 10000 = 100%
    pub treasury: Address,
    pub exemptions: Vec<Address>, // fee exemption list
}


pub fn set_platform_fee(env: &Env, new_fee_bps: i128) {
    require_governance(env);
    if new_fee_bps < 0 || new_fee_bps > 10000 {
        panic!("Invalid fee percentage");
    }
    env.storage().set("platform_fee_bps", &new_fee_bps);
}

pub fn set_treasury(env: &Env, treasury: Address) {
    require_governance(env);
    env.storage().set("treasury", &treasury);
}

pub fn add_fee_exemption(env: &Env, addr: Address) {
    require_governance(env);
    let mut exemptions: Vec<Address> = env.storage().get("exemptions").unwrap_or(Vec::new(env));
    if !exemptions.contains(&addr) {
        exemptions.push(addr);
        env.storage().set("exemptions", &exemptions);
    }
}

pub fn remove_fee_exemption(env: &Env, addr: Address) {
    require_governance(env);
    let mut exemptions: Vec<Address> = env.storage().get("exemptions").unwrap_or(Vec::new(env));
    exemptions.retain(|a| a != &addr);
    env.storage().set("exemptions", &exemptions);
}

pub fn apply_streaming_fee(env: &Env, withdrawer: Address, amount: i128) -> (i128, i128) {
    // Returns (net_amount, fee_amount)
    let fee_bps: i128 = env.storage().get("platform_fee_bps").unwrap_or(0);
    let treasury: Address = env.storage().get("treasury").unwrap_or_else(|| panic!("Treasury not set"));
    let exemptions: Vec<Address> = env.storage().get("exemptions").unwrap_or(Vec::new(env));

    if exemptions.contains(&withdrawer) || fee_bps == 0 {
        return (amount, 0);
    }

    let fee = (amount * fee_bps) / 10000;
    let net = amount - fee;

    // Route fee to treasury vault
    credit_treasury(env, &treasury, fee);

    // Emit event
    env.events().publish(
        (["streaming_fee", "collected"],),
        (withdrawer, fee, treasury),
    );

    (net, fee)
}

fn credit_treasury(env: &Env, treasury: &Address, fee: i128) {
    // Simplified: increment treasury balance
    let mut balance: i128 = env.storage().get(&format!("balance:{}", treasury)).unwrap_or(0);
    balance += fee;
    env.storage().set(&format!("balance:{}", treasury), &balance);
}


fn require_governance(env: &Env) {
    let caller = env.invoker();
    let governor: Address = env.storage().get("governor").unwrap();
    if caller != governor {
        panic!("Only governance can call this function");
    }
}
