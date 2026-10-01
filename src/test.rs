extern crate std;

use soroban_sdk::{
    testutils::{Address as _, ContractFunctionSet as _, MockAuth, MockAuthInvoke},
    xdr::{ScErrorCode, ScErrorType},
    Address, Env, Error, IntoVal, InvokeError, Val, Vec,
};

use crate::{PoolStatusAdmin, PoolStatusAdminClient};

/// The admin and status entrypoints of a Blend v2 pool.
mod pool {
    use soroban_sdk::{contract, contractimpl, contracttype, Address, Env};

    #[contracttype]
    enum Key {
        Admin,
        ProposedAdmin,
        Status,
    }

    #[contract]
    pub struct Pool;

    #[contractimpl]
    impl Pool {
        pub fn __constructor(e: &Env, admin: Address) {
            e.storage().instance().set(&Key::Admin, &admin);
            e.storage().instance().set(&Key::Status, &6u32);
        }

        pub fn propose_admin(e: &Env, new_admin: Address) {
            Self::admin(e).require_auth();
            e.storage().temporary().set(&Key::ProposedAdmin, &new_admin);
        }

        pub fn accept_admin(e: &Env) {
            let proposed: Address = e.storage().temporary().get(&Key::ProposedAdmin).unwrap();
            proposed.require_auth();
            e.storage().instance().set(&Key::Admin, &proposed);
        }

        pub fn set_status(e: &Env, pool_status: u32) {
            Self::admin(e).require_auth();
            e.storage().instance().set(&Key::Status, &pool_status);
        }

        pub fn admin(e: &Env) -> Address {
            e.storage().instance().get(&Key::Admin).unwrap()
        }

        pub fn status(e: &Env) -> u32 {
            e.storage().instance().get(&Key::Status).unwrap()
        }
    }
}

use pool::{Pool, PoolClient};

type Failure = Result<Error, InvokeError>;

const UNAUTHORIZED: Failure = Ok(Error::from_type_and_code(
    ScErrorType::Context,
    ScErrorCode::InvalidAction,
));
const OWNER_NOT_SET: Failure = Ok(Error::from_contract_error(2100));

/// A status admin and a pool still run by its original admin.
struct Fixture<'a> {
    e: Env,
    admin: PoolStatusAdminClient<'a>,
    pool: PoolClient<'a>,
    owner: Address,
}

impl<'a> Fixture<'a> {
    fn new() -> Self {
        let e = Env::default();
        let owner = Address::generate(&e);
        let admin_id = e.register(PoolStatusAdmin, (owner.clone(),));
        let pool = Self::new_pool(&e);

        Fixture {
            admin: PoolStatusAdminClient::new(&e, &admin_id),
            pool,
            owner,
            e,
        }
    }

    fn new_pool(e: &Env) -> PoolClient<'a> {
        let pool_id = e.register(Pool, (Address::generate(e),));
        PoolClient::new(e, &pool_id)
    }

    /// Authorizes `signer` for exactly one call to this contract and nothing
    /// beneath it.
    fn sign(&self, signer: &Address, fn_name: &str, args: Vec<Val>) {
        self.e.mock_auths(&[MockAuth {
            address: signer,
            invoke: &MockAuthInvoke {
                contract: &self.admin.address,
                fn_name,
                args,
                sub_invokes: &[],
            },
        }]);
    }

    /// The pool's admin proposes this contract as the new admin.
    fn propose(&self, pool: &PoolClient) {
        self.e.mock_auths(&[MockAuth {
            address: &pool.admin(),
            invoke: &MockAuthInvoke {
                contract: &pool.address,
                fn_name: "propose_admin",
                args: (&self.admin.address,).into_val(&self.e),
                sub_invokes: &[],
            },
        }]);
        pool.propose_admin(&self.admin.address);
    }

    fn try_accept(&self, signer: &Address, pool: &PoolClient) -> Result<(), Failure> {
        self.sign(signer, "accept_admin", (&pool.address,).into_val(&self.e));
        self.admin
            .try_accept_admin(&pool.address)
            .map(Result::unwrap)
    }

    fn try_set_status(
        &self,
        signer: &Address,
        pool: &PoolClient,
        status: u32,
    ) -> Result<(), Failure> {
        self.sign(
            signer,
            "set_status",
            (&pool.address, status).into_val(&self.e),
        );
        self.admin
            .try_set_status(&pool.address, &status)
            .map(Result::unwrap)
    }
}

#[test]
fn constructor_sets_owner() {
    let f = Fixture::new();

    assert_eq!(f.admin.get_owner(), Some(f.owner.clone()));
}

#[test]
fn owner_accepts_admin_and_sets_status() {
    let f = Fixture::new();

    f.propose(&f.pool);
    assert_eq!(f.try_accept(&f.owner, &f.pool), Ok(()));
    assert_eq!(f.pool.admin(), f.admin.address);

    for status in [4, 2, 3, 0] {
        assert_eq!(f.try_set_status(&f.owner, &f.pool, status), Ok(()));
        assert_eq!(f.pool.status(), status);
    }
}

#[test]
fn owner_manages_several_pools() {
    let f = Fixture::new();
    let other_pool = Fixture::new_pool(&f.e);

    for pool in [&f.pool, &other_pool] {
        f.propose(pool);
        f.try_accept(&f.owner, pool).unwrap();
    }
    f.try_set_status(&f.owner, &f.pool, 4).unwrap();
    f.try_set_status(&f.owner, &other_pool, 2).unwrap();

    assert_eq!(f.pool.status(), 4);
    assert_eq!(other_pool.status(), 2);
}

#[test]
fn other_signer_cannot_accept_or_set_status() {
    let f = Fixture::new();
    let other = Address::generate(&f.e);
    let pool_admin = f.pool.admin();

    f.propose(&f.pool);
    assert_eq!(f.try_accept(&other, &f.pool), Err(UNAUTHORIZED));
    assert_eq!(f.pool.admin(), pool_admin);

    f.try_accept(&f.owner, &f.pool).unwrap();
    assert_eq!(f.try_set_status(&other, &f.pool, 4), Err(UNAUTHORIZED));
    assert_eq!(f.pool.status(), 6);
}

#[test]
fn set_status_needs_the_admin_role() {
    let f = Fixture::new();

    assert_eq!(f.try_set_status(&f.owner, &f.pool, 4), Err(UNAUTHORIZED));
    assert_eq!(f.pool.status(), 6);
}

#[test]
fn renounce_ends_status_control() {
    let f = Fixture::new();

    f.propose(&f.pool);
    f.try_accept(&f.owner, &f.pool).unwrap();
    f.sign(&f.owner, "renounce_ownership", Vec::new(&f.e));
    f.admin.renounce_ownership();

    assert_eq!(f.admin.get_owner(), None);
    assert_eq!(f.try_set_status(&f.owner, &f.pool, 4), Err(OWNER_NOT_SET));
    assert_eq!(f.pool.status(), 6);
}

#[test]
fn new_owner_takes_over_after_transfer() {
    let f = Fixture::new();
    let new_owner = Address::generate(&f.e);

    f.propose(&f.pool);
    f.e.mock_all_auths();
    f.admin.transfer_ownership(&new_owner, &100);
    f.admin.accept_ownership();
    assert_eq!(f.admin.get_owner(), Some(new_owner.clone()));

    assert_eq!(f.try_accept(&f.owner, &f.pool), Err(UNAUTHORIZED));
    assert_eq!(f.try_accept(&new_owner, &f.pool), Ok(()));
    assert_eq!(f.try_set_status(&new_owner, &f.pool, 4), Ok(()));
    assert_eq!(f.pool.status(), 4);
}

#[test]
fn exposes_no_other_admin_entrypoints() {
    let f = Fixture::new();

    f.e.as_contract(&f.admin.address, || {
        assert!(PoolStatusAdmin
            .call("get_owner", f.e.clone(), &[])
            .is_some());
        for function in [
            "propose_admin",
            "update_pool",
            "queue_set_reserve",
            "cancel_set_reserve",
            "set_reserve",
            "set_emissions_config",
            "upgrade",
            "__check_auth",
        ] {
            assert!(PoolStatusAdmin.call(function, f.e.clone(), &[]).is_none());
        }
    });
}
