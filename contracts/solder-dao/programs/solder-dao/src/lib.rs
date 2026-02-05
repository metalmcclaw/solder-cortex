use anchor_lang::prelude::*;

declare_id!("zcAKcWaZFoueQ4XB7AYoqNjVcu6WtLi7dx5kQ7mCsbJ");

/// Solder Cortex: The Memory Layer for the Agent Economy
/// 
/// This program provides on-chain primitives for AI agents to:
/// - Register their identity with staked collateral
/// - Log immutable memories (actions, decisions, observations)
/// - Receive attestations from other agents/oracles
/// - Build verifiable reputation based on track record
#[program]
pub mod solder_cortex {
    use super::*;

    // =========================================================================
    // Agent Registry Instructions
    // =========================================================================

    /// Register a new agent on-chain
    /// 
    /// Creates an AgentRegistry account with initial metadata and stake.
    /// The agent_id should be a unique identifier (e.g., hash of pubkey or UUID).
    pub fn register_agent(
        ctx: Context<RegisterAgent>,
        agent_id: [u8; 32],
        metadata_uri: String,
        initial_stake: u64,
    ) -> Result<()> {
        require!(metadata_uri.len() <= 200, CortexError::MetadataUriTooLong);
        require!(initial_stake >= MIN_AGENT_STAKE, CortexError::InsufficientStake);

        let agent = &mut ctx.accounts.agent;
        agent.bump = ctx.bumps.agent;
        agent.owner = ctx.accounts.owner.key();
        agent.agent_id = agent_id;
        agent.metadata_uri = metadata_uri;
        agent.created_at = Clock::get()?.unix_timestamp;
        agent.memory_count = 0;
        agent.attestation_score = 0;
        agent.dispute_count = 0;
        agent.stake = initial_stake;
        agent.status = AgentStatus::Active;

        // Transfer stake from owner to agent PDA
        let cpi_context = CpiContext::new(
            ctx.accounts.system_program.to_account_info(),
            anchor_lang::system_program::Transfer {
                from: ctx.accounts.owner.to_account_info(),
                to: ctx.accounts.agent.to_account_info(),
            },
        );
        anchor_lang::system_program::transfer(cpi_context, initial_stake)?;

        emit!(AgentRegistered {
            agent: agent.key(),
            owner: agent.owner,
            agent_id,
            stake: initial_stake,
            timestamp: agent.created_at,
        });

        Ok(())
    }

    /// Update agent metadata
    pub fn update_agent(
        ctx: Context<UpdateAgent>,
        metadata_uri: Option<String>,
        status: Option<AgentStatus>,
    ) -> Result<()> {
        let agent = &mut ctx.accounts.agent;

        if let Some(uri) = metadata_uri {
            require!(uri.len() <= 200, CortexError::MetadataUriTooLong);
            agent.metadata_uri = uri;
        }

        if let Some(new_status) = status {
            agent.status = new_status;
        }

        Ok(())
    }

    /// Add stake to an agent
    pub fn add_stake(ctx: Context<AddStake>, amount: u64) -> Result<()> {
        let agent = &mut ctx.accounts.agent;

        let cpi_context = CpiContext::new(
            ctx.accounts.system_program.to_account_info(),
            anchor_lang::system_program::Transfer {
                from: ctx.accounts.owner.to_account_info(),
                to: ctx.accounts.agent.to_account_info(),
            },
        );
        anchor_lang::system_program::transfer(cpi_context, amount)?;

        agent.stake = agent.stake.checked_add(amount).ok_or(CortexError::Overflow)?;

        Ok(())
    }

    // =========================================================================
    // Memory Instructions
    // =========================================================================

    /// Log a new memory entry
    /// 
    /// Creates an immutable record of an agent action, decision, or observation.
    /// Content is stored off-chain; only hashes are stored on-chain.
    pub fn log_memory(
        ctx: Context<LogMemory>,
        memory_type: MemoryType,
        content_hash: [u8; 32],
        content_uri: String,
        context_hash: [u8; 32],
    ) -> Result<()> {
        require!(content_uri.len() <= 200, CortexError::ContentUriTooLong);
        
        let agent = &mut ctx.accounts.agent;
        require!(agent.status == AgentStatus::Active, CortexError::AgentNotActive);

        let memory = &mut ctx.accounts.memory;
        memory.bump = ctx.bumps.memory;
        memory.agent = agent.key();
        memory.sequence = agent.memory_count;
        memory.timestamp = Clock::get()?.unix_timestamp;
        memory.memory_type = memory_type.clone();
        memory.content_hash = content_hash;
        memory.content_uri = content_uri;
        memory.context_hash = context_hash;
        memory.attestation_count = 0;
        memory.dispute_count = 0;

        agent.memory_count = agent.memory_count.checked_add(1).ok_or(CortexError::Overflow)?;

        emit!(MemoryLogged {
            agent: agent.key(),
            memory: memory.key(),
            sequence: memory.sequence,
            memory_type,
            content_hash,
            timestamp: memory.timestamp,
        });

        Ok(())
    }

    // =========================================================================
    // Attestation Instructions
    // =========================================================================

    /// Attest to a memory entry
    /// 
    /// Another agent or oracle can vouch for the validity of a memory.
    /// Attestations can be staked for stronger guarantees.
    pub fn attest(
        ctx: Context<Attest>,
        attestation_type: AttestationType,
        evidence_hash: [u8; 32],
        stake_amount: u64,
    ) -> Result<()> {
        let memory = &mut ctx.accounts.memory;
        let attestation = &mut ctx.accounts.attestation;

        attestation.bump = ctx.bumps.attestation;
        attestation.memory = memory.key();
        attestation.attester = ctx.accounts.attester.key();
        attestation.attestation_type = attestation_type.clone();
        attestation.timestamp = Clock::get()?.unix_timestamp;
        attestation.stake = stake_amount;
        attestation.evidence_hash = evidence_hash;
        attestation.status = AttestationStatus::Active;

        // Transfer stake if provided
        if stake_amount > 0 {
            let cpi_context = CpiContext::new(
                ctx.accounts.system_program.to_account_info(),
                anchor_lang::system_program::Transfer {
                    from: ctx.accounts.attester.to_account_info(),
                    to: attestation.to_account_info(),
                },
            );
            anchor_lang::system_program::transfer(cpi_context, stake_amount)?;
        }

        // Update memory attestation count
        match attestation_type {
            AttestationType::Witnessed | AttestationType::Verified => {
                memory.attestation_count = memory.attestation_count.saturating_add(1);
            }
            AttestationType::Disputed => {
                memory.dispute_count = memory.dispute_count.saturating_add(1);
            }
        }

        emit!(AttestationCreated {
            memory: memory.key(),
            attester: attestation.attester,
            attestation_type,
            stake: stake_amount,
            timestamp: attestation.timestamp,
        });

        Ok(())
    }

    /// Resolve a dispute on an attestation
    /// 
    /// Called by an authorized resolver (oracle or DAO).
    /// Slashes stake from the losing party.
    pub fn resolve_dispute(
        ctx: Context<ResolveDispute>,
        resolution: DisputeResolution,
    ) -> Result<()> {
        let attestation = &mut ctx.accounts.attestation;
        
        require!(
            attestation.attestation_type == AttestationType::Disputed,
            CortexError::NotDisputed
        );
        require!(
            attestation.status == AttestationStatus::Active,
            CortexError::AlreadyResolved
        );

        attestation.status = match resolution {
            DisputeResolution::AttesterCorrect => AttestationStatus::Validated,
            DisputeResolution::AttesterIncorrect => AttestationStatus::Slashed,
            DisputeResolution::Inconclusive => AttestationStatus::Withdrawn,
        };

        // TODO: Implement stake slashing/refund logic
        // For MVP, just emit the event

        emit!(DisputeResolved {
            attestation: attestation.key(),
            resolution,
            timestamp: Clock::get()?.unix_timestamp,
        });

        Ok(())
    }
}

// =============================================================================
// Constants
// =============================================================================

/// Minimum stake required to register an agent (0.1 SOL)
pub const MIN_AGENT_STAKE: u64 = 100_000_000;

// =============================================================================
// Account Structures
// =============================================================================

/// Agent identity registered on-chain
#[account]
#[derive(Default)]
pub struct AgentRegistry {
    /// PDA bump seed
    pub bump: u8,
    /// Owner wallet (can update agent)
    pub owner: Pubkey,
    /// Unique agent identifier
    pub agent_id: [u8; 32],
    /// Off-chain metadata URI (IPFS/Arweave)
    pub metadata_uri: String,
    /// Registration timestamp
    pub created_at: i64,
    /// Total memories logged
    pub memory_count: u64,
    /// Cumulative attestation score
    pub attestation_score: i64,
    /// Number of disputes against this agent
    pub dispute_count: u64,
    /// SOL staked as collateral
    pub stake: u64,
    /// Current status
    pub status: AgentStatus,
}

impl AgentRegistry {
    pub const MAX_SIZE: usize = 8 + // discriminator
        1 +     // bump
        32 +    // owner
        32 +    // agent_id
        4 + 200 + // metadata_uri (String prefix + max chars)
        8 +     // created_at
        8 +     // memory_count
        8 +     // attestation_score
        8 +     // dispute_count
        8 +     // stake
        1 +     // status
        64;     // padding
}

/// Individual memory entry
#[account]
#[derive(Default)]
pub struct MemoryEntry {
    /// PDA bump seed
    pub bump: u8,
    /// Parent agent
    pub agent: Pubkey,
    /// Sequence number (monotonic)
    pub sequence: u64,
    /// Unix timestamp
    pub timestamp: i64,
    /// Type of memory
    pub memory_type: MemoryType,
    /// Hash of full content (stored off-chain)
    pub content_hash: [u8; 32],
    /// URI to full content
    pub content_uri: String,
    /// Hash of input context (for replay verification)
    pub context_hash: [u8; 32],
    /// Number of positive attestations
    pub attestation_count: u16,
    /// Number of disputes
    pub dispute_count: u16,
}

impl MemoryEntry {
    pub const MAX_SIZE: usize = 8 + // discriminator
        1 +     // bump
        32 +    // agent
        8 +     // sequence
        8 +     // timestamp
        1 +     // memory_type
        32 +    // content_hash
        4 + 200 + // content_uri
        32 +    // context_hash
        2 +     // attestation_count
        2 +     // dispute_count
        32;     // padding
}

/// Third-party attestation of a memory
#[account]
#[derive(Default)]
pub struct Attestation {
    /// PDA bump seed
    pub bump: u8,
    /// Memory being attested
    pub memory: Pubkey,
    /// Attester (agent or oracle)
    pub attester: Pubkey,
    /// Type of attestation
    pub attestation_type: AttestationType,
    /// Unix timestamp
    pub timestamp: i64,
    /// SOL staked on this attestation
    pub stake: u64,
    /// Hash of evidence (optional)
    pub evidence_hash: [u8; 32],
    /// Current status
    pub status: AttestationStatus,
}

impl Attestation {
    pub const MAX_SIZE: usize = 8 + // discriminator
        1 +     // bump
        32 +    // memory
        32 +    // attester
        1 +     // attestation_type
        8 +     // timestamp
        8 +     // stake
        32 +    // evidence_hash
        1 +     // status
        16;     // padding
}

// =============================================================================
// Enums
// =============================================================================

#[derive(AnchorSerialize, AnchorDeserialize, Clone, PartialEq, Eq, Default)]
pub enum AgentStatus {
    #[default]
    Active,
    Suspended,
    Retired,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, PartialEq, Eq, Default)]
pub enum MemoryType {
    #[default]
    Action,
    Decision,
    Observation,
    Error,
    Milestone,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, PartialEq, Eq, Default)]
pub enum AttestationType {
    #[default]
    Witnessed,
    Verified,
    Disputed,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, PartialEq, Eq, Default)]
pub enum AttestationStatus {
    #[default]
    Active,
    Validated,
    Slashed,
    Withdrawn,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, PartialEq, Eq)]
pub enum DisputeResolution {
    AttesterCorrect,
    AttesterIncorrect,
    Inconclusive,
}

// =============================================================================
// Context Structs
// =============================================================================

#[derive(Accounts)]
#[instruction(agent_id: [u8; 32])]
pub struct RegisterAgent<'info> {
    #[account(
        init,
        payer = owner,
        space = AgentRegistry::MAX_SIZE,
        seeds = [b"agent", owner.key().as_ref(), agent_id.as_ref()],
        bump
    )]
    pub agent: Account<'info, AgentRegistry>,

    #[account(mut)]
    pub owner: Signer<'info>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct UpdateAgent<'info> {
    #[account(
        mut,
        seeds = [b"agent", agent.owner.as_ref(), agent.agent_id.as_ref()],
        bump = agent.bump,
        has_one = owner
    )]
    pub agent: Account<'info, AgentRegistry>,

    pub owner: Signer<'info>,
}

#[derive(Accounts)]
pub struct AddStake<'info> {
    #[account(
        mut,
        seeds = [b"agent", agent.owner.as_ref(), agent.agent_id.as_ref()],
        bump = agent.bump,
        has_one = owner
    )]
    pub agent: Account<'info, AgentRegistry>,

    #[account(mut)]
    pub owner: Signer<'info>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct LogMemory<'info> {
    #[account(
        mut,
        seeds = [b"agent", agent.owner.as_ref(), agent.agent_id.as_ref()],
        bump = agent.bump,
        has_one = owner
    )]
    pub agent: Account<'info, AgentRegistry>,

    #[account(
        init,
        payer = owner,
        space = MemoryEntry::MAX_SIZE,
        seeds = [b"memory", agent.key().as_ref(), &agent.memory_count.to_le_bytes()],
        bump
    )]
    pub memory: Account<'info, MemoryEntry>,

    #[account(mut)]
    pub owner: Signer<'info>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct Attest<'info> {
    #[account(mut)]
    pub memory: Account<'info, MemoryEntry>,

    #[account(
        init,
        payer = attester,
        space = Attestation::MAX_SIZE,
        seeds = [b"attestation", memory.key().as_ref(), attester.key().as_ref()],
        bump
    )]
    pub attestation: Account<'info, Attestation>,

    #[account(mut)]
    pub attester: Signer<'info>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct ResolveDispute<'info> {
    #[account(mut)]
    pub attestation: Account<'info, Attestation>,

    /// Authorized resolver (oracle or DAO)
    pub resolver: Signer<'info>,
}

// =============================================================================
// Events
// =============================================================================

#[event]
pub struct AgentRegistered {
    pub agent: Pubkey,
    pub owner: Pubkey,
    pub agent_id: [u8; 32],
    pub stake: u64,
    pub timestamp: i64,
}

#[event]
pub struct MemoryLogged {
    pub agent: Pubkey,
    pub memory: Pubkey,
    pub sequence: u64,
    pub memory_type: MemoryType,
    pub content_hash: [u8; 32],
    pub timestamp: i64,
}

#[event]
pub struct AttestationCreated {
    pub memory: Pubkey,
    pub attester: Pubkey,
    pub attestation_type: AttestationType,
    pub stake: u64,
    pub timestamp: i64,
}

#[event]
pub struct DisputeResolved {
    pub attestation: Pubkey,
    pub resolution: DisputeResolution,
    pub timestamp: i64,
}

// =============================================================================
// Errors
// =============================================================================

#[error_code]
pub enum CortexError {
    #[msg("Metadata URI exceeds maximum length of 200 characters")]
    MetadataUriTooLong,
    #[msg("Content URI exceeds maximum length of 200 characters")]
    ContentUriTooLong,
    #[msg("Insufficient stake amount")]
    InsufficientStake,
    #[msg("Agent is not active")]
    AgentNotActive,
    #[msg("Arithmetic overflow")]
    Overflow,
    #[msg("Attestation is not a dispute")]
    NotDisputed,
    #[msg("Dispute already resolved")]
    AlreadyResolved,
}
