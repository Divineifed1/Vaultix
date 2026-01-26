#![no_std]
use soroban_sdk::{
    contract, contracterror, contractimpl, contracttype, symbol_short, Address, Env, Symbol, Vec,
};

// Milestone status tracking
#[contracttype]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum MilestoneStatus {
    Pending,
    Released,
    Disputed,
}

// Individual milestone in an escrow
#[contracttype]
#[derive(Clone, Debug)]
pub struct Milestone {
    pub amount: i128,
    pub status: MilestoneStatus,
    pub description: Symbol,
}

// Overall escrow status
#[contracttype]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum EscrowStatus {
    Active,
    Completed,
    Cancelled,
}

// Main escrow structure
#[contracttype]
#[derive(Clone, Debug)]
pub struct Escrow {
    pub depositor: Address,
    pub recipient: Address,
    pub total_amount: i128,
    pub total_released: i128,
    pub milestones: Vec<Milestone>,
    pub status: EscrowStatus,
}

// Contract error types
#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum Error {
    EscrowNotFound = 1,
    EscrowAlreadyExists = 2,
    MilestoneNotFound = 3,
    MilestoneAlreadyReleased = 4,
    UnauthorizedAccess = 5,
    InvalidMilestoneAmount = 6,
    TotalAmountMismatch = 7,
    InsufficientBalance = 8,
    EscrowNotActive = 9,
    VectorTooLarge = 10,
    MilestoneNotDisputed = 11,
}

#[contract]
pub struct VaultixEscrow;

#[contractimpl]
impl VaultixEscrow {
    /// Creates a new escrow with milestone-based payment releases.
    pub fn create_escrow(
        env: Env,
        escrow_id: u64,
        depositor: Address,
        recipient: Address,
        milestones: Vec<Milestone>,
    ) -> Result<(), Error> {
        // Authenticate the depositor
        depositor.require_auth();

        // Check if escrow already exists
        let storage_key = get_storage_key(escrow_id);
        if env.storage().persistent().has(&storage_key) {
            return Err(Error::EscrowAlreadyExists);
        }

        // Validate milestones and calculate total
        let total_amount = validate_milestones(&milestones)?;

        // Initialize all milestones to Pending status
        let mut initialized_milestones = Vec::new(&env);
        for milestone in milestones.iter() {
            let mut m = milestone.clone();
            m.status = MilestoneStatus::Pending;
            initialized_milestones.push_back(m);
        }

        // Create the escrow
        let escrow = Escrow {
            depositor: depositor.clone(),
            recipient: recipient.clone(),
            total_amount,
            total_released: 0,
            milestones: initialized_milestones,
            status: EscrowStatus::Active,
        };

        // Save to persistent storage
        env.storage().persistent().set(&storage_key, &escrow);

        // Emit event
        env.events().publish(
            (symbol_short!("create"), escrow_id, depositor, recipient),
            total_amount,
        );

        Ok(())
    }

    /// Releases a specific milestone payment to the recipient.
    pub fn release_milestone(env: Env, escrow_id: u64, milestone_index: u32) -> Result<(), Error> {
        let storage_key = get_storage_key(escrow_id);

        // Load escrow from storage
        let mut escrow: Escrow = env
            .storage()
            .persistent()
            .get(&storage_key)
            .ok_or(Error::EscrowNotFound)?;

        // Verify authorization
        escrow.depositor.require_auth();

        // Check escrow is active
        if escrow.status != EscrowStatus::Active {
            return Err(Error::EscrowNotActive);
        }

        // Verify milestone index is valid
        if milestone_index >= escrow.milestones.len() {
            return Err(Error::MilestoneNotFound);
        }

        // Get the milestone
        let mut milestone = escrow
            .milestones
            .get(milestone_index)
            .ok_or(Error::MilestoneNotFound)?;

        // Check if already released
        if milestone.status == MilestoneStatus::Released {
            return Err(Error::MilestoneAlreadyReleased);
        }

        // Update milestone status
        milestone.status = MilestoneStatus::Released;
        escrow.milestones.set(milestone_index, milestone.clone());

        // Update total released with overflow protection
        escrow.total_released = escrow
            .total_released
            .checked_add(milestone.amount)
            .ok_or(Error::InvalidMilestoneAmount)?;

        // Save updated escrow
        env.storage().persistent().set(&storage_key, &escrow);

        // Emit event
        env.events().publish(
            (symbol_short!("release"), escrow_id, milestone_index),
            milestone.amount,
        );

        Ok(())
    }

    /// Raises a dispute for a specific milestone.
    pub fn raise_dispute(
        env: Env,
        escrow_id: u64,
        milestone_index: u32,
        caller: Address,
    ) -> Result<(), Error> {
        caller.require_auth();
        let storage_key = get_storage_key(escrow_id);

        let mut escrow: Escrow = env
            .storage()
            .persistent()
            .get(&storage_key)
            .ok_or(Error::EscrowNotFound)?;

        if caller != escrow.depositor && caller != escrow.recipient {
            return Err(Error::UnauthorizedAccess);
        }

        let mut milestone = escrow
            .milestones
            .get(milestone_index)
            .ok_or(Error::MilestoneNotFound)?;

        if milestone.status != MilestoneStatus::Pending {
            return Err(Error::MilestoneAlreadyReleased);
        }

        milestone.status = MilestoneStatus::Disputed;
        escrow.milestones.set(milestone_index, milestone);
        env.storage().persistent().set(&storage_key, &escrow);

        // Emit event
        env.events().publish(
            (symbol_short!("dispute"), escrow_id, caller),
            milestone_index,
        );

        Ok(())
    }

    /// Resolves a dispute for a specific milestone.
    pub fn resolve_dispute(
        env: Env,
        escrow_id: u64,
        milestone_index: u32,
        winner: Address,
    ) -> Result<(), Error> {
        // In a real scenario, this would require admin auth
        let storage_key = get_storage_key(escrow_id);

        let mut escrow: Escrow = env
            .storage()
            .persistent()
            .get(&storage_key)
            .ok_or(Error::EscrowNotFound)?;

        let mut milestone = escrow
            .milestones
            .get(milestone_index)
            .ok_or(Error::MilestoneNotFound)?;

        if milestone.status != MilestoneStatus::Disputed {
            return Err(Error::MilestoneNotDisputed);
        }

        milestone.status = MilestoneStatus::Released;
        escrow.milestones.set(milestone_index, milestone.clone());

        if winner == escrow.recipient {
            escrow.total_released = escrow
                .total_released
                .checked_add(milestone.amount)
                .ok_or(Error::InvalidMilestoneAmount)?;
        }

        env.storage().persistent().set(&storage_key, &escrow);

        // Emit event
        env.events().publish(
            (symbol_short!("resolve"), escrow_id, winner),
            milestone.amount,
        );

        Ok(())
    }

    /// Retrieves escrow details.
    pub fn get_escrow(env: Env, escrow_id: u64) -> Result<Escrow, Error> {
        let storage_key = get_storage_key(escrow_id);
        env.storage()
            .persistent()
            .get(&storage_key)
            .ok_or(Error::EscrowNotFound)
    }

    /// Cancels an escrow before any milestones are released.
    pub fn cancel_escrow(env: Env, escrow_id: u64) -> Result<(), Error> {
        let storage_key = get_storage_key(escrow_id);

        let mut escrow: Escrow = env
            .storage()
            .persistent()
            .get(&storage_key)
            .ok_or(Error::EscrowNotFound)?;

        // Verify authorization
        escrow.depositor.require_auth();

        // Verify no milestones have been released
        if escrow.total_released > 0 {
            return Err(Error::MilestoneAlreadyReleased);
        }

        // Update status
        escrow.status = EscrowStatus::Cancelled;
        env.storage().persistent().set(&storage_key, &escrow);

        Ok(())
    }

    /// Marks an escrow as completed after all milestones are released.
    pub fn complete_escrow(env: Env, escrow_id: u64) -> Result<(), Error> {
        let storage_key = get_storage_key(escrow_id);

        let mut escrow: Escrow = env
            .storage()
            .persistent()
            .get(&storage_key)
            .ok_or(Error::EscrowNotFound)?;

        // Verify authorization
        escrow.depositor.require_auth();

        // Verify all milestones are released
        if !verify_all_released(&escrow.milestones) {
            return Err(Error::EscrowNotActive);
        }

        // Update status
        escrow.status = EscrowStatus::Completed;
        env.storage().persistent().set(&storage_key, &escrow);

        Ok(())
    }
}

// Helper function to generate storage key
fn get_storage_key(escrow_id: u64) -> (Symbol, u64) {
    (symbol_short!("escrow"), escrow_id)
}

// Validates milestone vector and returns total amount
fn validate_milestones(milestones: &Vec<Milestone>) -> Result<i128, Error> {
    // Check vector size to prevent gas issues
    if milestones.len() > 20 {
        return Err(Error::VectorTooLarge);
    }

    let mut total: i128 = 0;

    // Validate each milestone and calculate total
    for milestone in milestones.iter() {
        if milestone.amount <= 0 {
            return Err(Error::InvalidMilestoneAmount);
        }

        total = total
            .checked_add(milestone.amount)
            .ok_or(Error::InvalidMilestoneAmount)?;
    }

    Ok(total)
}

// Checks if all milestones have been released
fn verify_all_released(milestones: &Vec<Milestone>) -> bool {
    for milestone in milestones.iter() {
        if milestone.status != MilestoneStatus::Released {
            return false;
        }
    }
    true
}

#[cfg(test)]
mod test;
