//! Admin for Blend pools that can only change their status, built on the
//! OpenZeppelin Stellar Contracts ownable module.
//!
//! The owner accepts the admin role on a pool once the pool's admin proposes
//! this contract, then sets that pool's status. The pools' other admin
//! functions stay out of reach.
#![no_std]

use soroban_sdk::{contract, contractclient, contractimpl, Address, Env};
use stellar_access::ownable::{self, Ownable};
use stellar_macros::only_owner;

/// The Blend pool functions this contract calls.
#[contractclient(name = "PoolClient")]
pub trait Pool {
    fn accept_admin(e: Env);
    fn set_status(e: Env, pool_status: u32);
}

#[contract]
pub struct PoolStatusAdmin;

#[contractimpl]
impl PoolStatusAdmin {
    /// Sets the owner.
    ///
    /// # Arguments
    ///
    /// * `owner` - The owner account.
    pub fn __constructor(e: &Env, owner: Address) {
        ownable::set_owner(e, &owner);
    }

    /// Accepts the admin role on `pool`, once the pool's admin has proposed
    /// this contract. Owner only.
    ///
    /// # Errors
    ///
    /// * refer to the pool's `accept_admin` errors.
    #[only_owner]
    pub fn accept_admin(e: &Env, pool: Address) {
        PoolClient::new(e, &pool).accept_admin();
    }

    /// Sets the status of `pool`. Owner only.
    ///
    /// # Arguments
    ///
    /// * `pool` - A pool this contract is the admin of.
    /// * `pool_status` - `0` admin active, `2` admin on-ice, `3` on-ice under
    ///   permissionless status updates, `4` admin frozen.
    ///
    /// # Errors
    ///
    /// * refer to the pool's `set_status` errors.
    #[only_owner]
    pub fn set_status(e: &Env, pool: Address, pool_status: u32) {
        PoolClient::new(e, &pool).set_status(&pool_status);
    }
}

#[contractimpl(contracttrait)]
impl Ownable for PoolStatusAdmin {}

#[cfg(test)]
mod test;
