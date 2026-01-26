use super::*;
use soroban_sdk::testutils::{Address as _, Events};
use soroban_sdk::{symbol_short, vec, Address, Env, IntoVal};

#[test]
fn test_create_and_get_escrow() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, VaultixEscrow);
    let client = VaultixEscrowClient::new(&env, &contract_id);

    let depositor = Address::generate(&env);
    let recipient = Address::generate(&env);
    let escrow_id = 1u64;

    // Create milestones
    let milestones = vec![
        &env,
        Milestone {
            amount: 3000,
            status: MilestoneStatus::Pending,
            description: symbol_short!("Design"),
        },
        Milestone {
            amount: 3000,
            status: MilestoneStatus::Pending,
            description: symbol_short!("Dev"),
        },
        Milestone {
            amount: 4000,
            status: MilestoneStatus::Pending,
            description: symbol_short!("Deploy"),
        },
    ];

    // Create escrow
    client.create_escrow(&escrow_id, &depositor, &recipient, &milestones);

    // Retrieve escrow
    let escrow = client.get_escrow(&escrow_id);
    assert_eq!(escrow.depositor, depositor);
    assert_eq!(escrow.recipient, recipient);
    assert_eq!(escrow.total_amount, 10000);
    assert_eq!(escrow.total_released, 0);
    assert_eq!(escrow.status, EscrowStatus::Active);
    assert_eq!(escrow.milestones.len(), 3);

    // Verify Create Event
    let events = env.events().all();
    assert_eq!(events.len(), 1);
    let event = events.get(0).unwrap();
    assert_eq!(event.0, contract_id);
    assert_eq!(
        event.1,
        (
            symbol_short!("create"),
            escrow_id,
            depositor.clone(),
            recipient.clone()
        )
            .into_val(&env)
    );
    let total_amount: i128 = event.2.into_val(&env);
    assert_eq!(total_amount, 10000);
}

#[test]
fn test_release_milestone() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, VaultixEscrow);
    let client = VaultixEscrowClient::new(&env, &contract_id);

    let depositor = Address::generate(&env);
    let recipient = Address::generate(&env);
    let escrow_id = 2u64;

    let milestones = vec![
        &env,
        Milestone {
            amount: 5000,
            status: MilestoneStatus::Pending,
            description: symbol_short!("Phase1"),
        },
        Milestone {
            amount: 5000,
            status: MilestoneStatus::Pending,
            description: symbol_short!("Phase2"),
        },
    ];

    client.create_escrow(&escrow_id, &depositor, &recipient, &milestones);

    // Release first milestone
    client.release_milestone(&escrow_id, &0);

    let escrow = client.get_escrow(&escrow_id);
    assert_eq!(escrow.total_released, 5000);
    assert_eq!(
        escrow.milestones.get(0).unwrap().status,
        MilestoneStatus::Released
    );

    // Verify Release Event
    let events = env.events().all();
    assert_eq!(events.len(), 2);
    let event = events.get(1).unwrap();
    assert_eq!(event.0, contract_id);
    assert_eq!(
        event.1,
        (symbol_short!("release"), escrow_id, 0u32).into_val(&env)
    );
    let amount: i128 = event.2.into_val(&env);
    assert_eq!(amount, 5000);
}

#[test]
fn test_dispute_flow() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, VaultixEscrow);
    let client = VaultixEscrowClient::new(&env, &contract_id);

    let depositor = Address::generate(&env);
    let recipient = Address::generate(&env);
    let escrow_id = 3u64;

    let milestones = vec![
        &env,
        Milestone {
            amount: 1000,
            status: MilestoneStatus::Pending,
            description: symbol_short!("Task"),
        },
    ];

    client.create_escrow(&escrow_id, &depositor, &recipient, &milestones);

    // Raise Dispute
    client.raise_dispute(&escrow_id, &0, &depositor);
    let escrow = client.get_escrow(&escrow_id);
    assert_eq!(
        escrow.milestones.get(0).unwrap().status,
        MilestoneStatus::Disputed
    );

    // Verify Dispute Event
    let events = env.events().all();
    let event = events.get(1).unwrap();
    assert_eq!(event.0, contract_id);
    assert_eq!(
        event.1,
        (symbol_short!("dispute"), escrow_id, depositor.clone()).into_val(&env)
    );
    let milestone_index: u32 = event.2.into_val(&env);
    assert_eq!(milestone_index, 0);

    // Resolve Dispute
    client.resolve_dispute(&escrow_id, &0, &recipient);
    let escrow = client.get_escrow(&escrow_id);
    assert_eq!(
        escrow.milestones.get(0).unwrap().status,
        MilestoneStatus::Released
    );
    assert_eq!(escrow.total_released, 1000);

    // Verify Resolve Event
    let events = env.events().all();
    let event = events.get(2).unwrap();
    assert_eq!(event.0, contract_id);
    assert_eq!(
        event.1,
        (symbol_short!("resolve"), escrow_id, recipient.clone()).into_val(&env)
    );
    let amount: i128 = event.2.into_val(&env);
    assert_eq!(amount, 1000);
}

#[test]
fn test_complete_escrow() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, VaultixEscrow);
    let client = VaultixEscrowClient::new(&env, &contract_id);

    let depositor = Address::generate(&env);
    let recipient = Address::generate(&env);
    let escrow_id = 3u64;

    let milestones = vec![
        &env,
        Milestone {
            amount: 5000,
            status: MilestoneStatus::Pending,
            description: symbol_short!("Task1"),
        },
        Milestone {
            amount: 5000,
            status: MilestoneStatus::Pending,
            description: symbol_short!("Task2"),
        },
    ];

    client.create_escrow(&escrow_id, &depositor, &recipient, &milestones);

    // Release all milestones
    client.release_milestone(&escrow_id, &0);
    client.release_milestone(&escrow_id, &1);

    // Complete the escrow
    client.complete_escrow(&escrow_id);

    let escrow = client.get_escrow(&escrow_id);
    assert_eq!(escrow.status, EscrowStatus::Completed);
    assert_eq!(escrow.total_released, 10000);
}

#[test]
fn test_cancel_escrow() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, VaultixEscrow);
    let client = VaultixEscrowClient::new(&env, &contract_id);

    let depositor = Address::generate(&env);
    let recipient = Address::generate(&env);
    let escrow_id = 4u64;

    let milestones = vec![
        &env,
        Milestone {
            amount: 10000,
            status: MilestoneStatus::Pending,
            description: symbol_short!("Work"),
        },
    ];

    client.create_escrow(&escrow_id, &depositor, &recipient, &milestones);

    // Cancel before any releases
    client.cancel_escrow(&escrow_id);

    let escrow = client.get_escrow(&escrow_id);
    assert_eq!(escrow.status, EscrowStatus::Cancelled);
}

#[test]
#[should_panic(expected = "Error(Contract, #2)")]
fn test_duplicate_escrow_id() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, VaultixEscrow);
    let client = VaultixEscrowClient::new(&env, &contract_id);

    let depositor = Address::generate(&env);
    let recipient = Address::generate(&env);
    let escrow_id = 5u64;

    let milestones = vec![
        &env,
        Milestone {
            amount: 1000,
            status: MilestoneStatus::Pending,
            description: symbol_short!("Test"),
        },
    ];

    client.create_escrow(&escrow_id, &depositor, &recipient, &milestones);
    // This should panic with Error #2 (EscrowAlreadyExists)
    client.create_escrow(&escrow_id, &depositor, &recipient, &milestones);
}

#[test]
#[should_panic(expected = "Error(Contract, #4)")]
fn test_double_release() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, VaultixEscrow);
    let client = VaultixEscrowClient::new(&env, &contract_id);

    let depositor = Address::generate(&env);
    let recipient = Address::generate(&env);
    let escrow_id = 6u64;

    let milestones = vec![
        &env,
        Milestone {
            amount: 1000,
            status: MilestoneStatus::Pending,
            description: symbol_short!("Task"),
        },
    ];

    client.create_escrow(&escrow_id, &depositor, &recipient, &milestones);
    client.release_milestone(&escrow_id, &0);
    // This should panic with Error #4 (MilestoneAlreadyReleased)
    client.release_milestone(&escrow_id, &0);
}

#[test]
#[should_panic(expected = "Error(Contract, #10)")]
fn test_too_many_milestones() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, VaultixEscrow);
    let client = VaultixEscrowClient::new(&env, &contract_id);

    let depositor = Address::generate(&env);
    let recipient = Address::generate(&env);
    let escrow_id = 7u64;

    // Create 21 milestones (exceeds max of 20)
    let mut milestones = Vec::new(&env);
    for _i in 0..21 {
        milestones.push_back(Milestone {
            amount: 100,
            status: MilestoneStatus::Pending,
            description: symbol_short!("Task"),
        });
    }

    // This should panic with Error #10 (VectorTooLarge)
    client.create_escrow(&escrow_id, &depositor, &recipient, &milestones);
}

#[test]
#[should_panic(expected = "Error(Contract, #12)")]
fn test_invalid_milestone_amount() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, VaultixEscrow);
    let client = VaultixEscrowClient::new(&env, &contract_id);

    let depositor = Address::generate(&env);
    let recipient = Address::generate(&env);
    let escrow_id = 8u64;

    let milestones = vec![
        &env,
        Milestone {
            amount: 0, // Invalid: zero amount
            status: MilestoneStatus::Pending,
            description: symbol_short!("Task"),
        },
    ];

    // This should panic with Error #11 (ZeroAmount)
    client.create_escrow(&escrow_id, &depositor, &recipient, &milestones);
}

#[test]
fn test_zero_amount_milestone_rejected() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, VaultixEscrow);
    let client = VaultixEscrowClient::new(&env, &contract_id);

    let depositor = Address::generate(&env);
    let recipient = Address::generate(&env);
    let escrow_id = 1u64;

    // Create milestones with one zero amount
    let milestones = vec![
        &env,
        Milestone {
            amount: 0, // Invalid: zero amount
            status: MilestoneStatus::Pending,
            description: symbol_short!("Test"),
        },
    ];

    // Attempt to create escrow with zero amount milestone
    let result = client.try_create_escrow(&escrow_id, &depositor, &recipient, &milestones);

    // Assert specific error is returned
    assert_eq!(result, Err(Ok(Error::ZeroAmount)));
}

#[test]
fn test_negative_amount_milestone_rejected() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, VaultixEscrow);
    let client = VaultixEscrowClient::new(&env, &contract_id);

    let depositor = Address::generate(&env);
    let recipient = Address::generate(&env);
    let escrow_id = 2u64;

    // Create milestones with negative amount
    let milestones = vec![
        &env,
        Milestone {
            amount: -1000, // Invalid: negative amount
            status: MilestoneStatus::Pending,
            description: symbol_short!("Test"),
        },
    ];

    // Attempt to create escrow
    let result = client.try_create_escrow(&escrow_id, &depositor, &recipient, &milestones);

    // Assert ZeroAmount error (covers negative case)
    assert_eq!(result, Err(Ok(Error::ZeroAmount)));
}

#[test]
fn test_self_dealing_rejected() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, VaultixEscrow);
    let client = VaultixEscrowClient::new(&env, &contract_id);

    let same_party = Address::generate(&env); // Same address for both
    let escrow_id = 3u64;

    // Create valid milestones
    let milestones = vec![
        &env,
        Milestone {
            amount: 5000,
            status: MilestoneStatus::Pending,
            description: symbol_short!("Task"),
        },
    ];

    // Attempt to create escrow where depositor == recipient
    let result = client.try_create_escrow(&escrow_id, &same_party, &same_party, &milestones);

    // Assert SelfDealing error
    assert_eq!(result, Err(Ok(Error::SelfDealing)));
}

#[test]
fn test_valid_escrow_creation_succeeds() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, VaultixEscrow);
    let client = VaultixEscrowClient::new(&env, &contract_id);

    let depositor = Address::generate(&env);
    let recipient = Address::generate(&env);
    let escrow_id = 4u64;

    // Valid milestones with positive amounts
    let milestones = vec![
        &env,
        Milestone {
            amount: 3000,
            status: MilestoneStatus::Pending,
            description: symbol_short!("Phase1"),
        },
        Milestone {
            amount: 7000,
            status: MilestoneStatus::Pending,
            description: symbol_short!("Phase2"),
        },
    ];

    // Create escrow - should succeed
    let result = client.try_create_escrow(&escrow_id, &depositor, &recipient, &milestones);

    // Assert success
    assert!(result.is_ok());

    // Verify escrow was created correctly
    let escrow = client.get_escrow(&escrow_id);
    assert_eq!(escrow.depositor, depositor);
    assert_eq!(escrow.recipient, recipient);
    assert_eq!(escrow.total_amount, 10000);
}
