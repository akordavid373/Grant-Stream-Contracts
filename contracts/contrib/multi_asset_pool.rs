use soroban_sdk::{Env, Address, panic};
use std::collections::HashMap;

#[derive(Clone)]
pub struct GrantPool {
    pub pool_id: String,
    pub balances: HashMap<Address, i128>, // Map of asset address → balance
    pub oracle: Address,                  // Oracle for price conversions
}

pub fn deposit(env: &Env, pool_id: String, asset: Address, amount: i128) {
    let mut pool: GrantPool = env.storage().get(&format!("pool:{}", pool_id))
        .unwrap_or(GrantPool {
            pool_id: pool_id.clone(),
            balances: HashMap::new(),
            oracle: Address::random(env), // placeholder
        });

    let entry = pool.balances.entry(asset.clone()).or_insert(0);
    *entry += amount;

    env.storage().set(&format!("pool:{}", pool_id), &pool);

    env.events().publish(
        (["pool", "deposit"],),
        (pool_id, asset, amount),
    );
}

pub fn withdraw(env: &Env, pool_id: String, grantee: Address, amount: i128, preferred_asset: Option<Address>) {
    let mut pool: GrantPool = env.storage().get(&format!("pool:{}", pool_id))
        .unwrap_or_else(|| panic!("Pool not found"));

    if amount <= 0 {
        panic!("Withdrawal amount must be positive");
    }

    if let Some(asset) = preferred_asset {
        // Single asset withdrawal based on oracle conversion
        let converted_amount = convert_via_oracle(env, &pool.oracle, amount, &asset);
        let balance = pool.balances.get_mut(&asset).unwrap_or_else(|| panic!("Asset not in pool"));
        if *balance < converted_amount {
            panic!("Insufficient asset balance in pool");
        }
        *balance -= converted_amount;

        env.events().publish(
            (["pool", "withdraw"],),
            (pool_id, grantee, asset, converted_amount),
        );
    } else {
        // Basket withdrawal: pro-rata across all assets
        let total_value = total_pool_value(env, &pool);
        if total_value < amount {
            panic!("Insufficient pool value");
        }

        for (asset, bal) in pool.balances.iter_mut() {
            let share = (*bal as i128 * amount) / total_value;
            *bal -= share;
            env.events().publish(
                (["pool", "withdraw"],),
                (pool_id.clone(), grantee.clone(), asset.clone(), share),
            );
        }
    }

    env.storage().set(&format!("pool:{}", pool_id), &pool);
}

fn convert_via_oracle(env: &Env, oracle: &Address, amount: i128, asset: &Address) -> i128 {
    // Simplified: fetch conversion rate from oracle
    let rate: i128 = env.storage().get(&format!("oracle:{}:rate", asset)).unwrap_or(1);
    amount * rate
}

fn total_pool_value(env: &Env, pool: &GrantPool) -> i128 {
    let mut total = 0;
    for (asset, bal) in pool.balances.iter() {
        let rate: i128 = env.storage().get(&format!("oracle:{}:rate", asset)).unwrap_or(1);
        total += bal * rate;
    }
    total
}
