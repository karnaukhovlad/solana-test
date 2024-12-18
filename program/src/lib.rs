use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{account_info::{next_account_info, AccountInfo}, entrypoint, entrypoint::ProgramResult, msg, program_error::ProgramError, pubkey::Pubkey, hash::{Hash, Hasher}, system_instruction};
use solana_program::borsh1::try_from_slice_unchecked;
use solana_program::hash::HASH_BYTES;
use solana_program::program::invoke_signed;
use solana_program::rent::Rent;
use solana_program::sysvar::Sysvar;

// #[derive(Debug, PartialEq)]
// /// All custom program instructions
// pub enum ProgramInstruction {
//     InitializeAccount,
//     InsertLeaf { },
// }


pub const PREFIX_PDA: &[u8] = b"merkle";
/// Initialization flag size
pub const INITIALIZED_BYTES: usize = 1;
pub const ROOT_HASH_BYTES: usize = HASH_BYTES;
pub const CHILD_BYTES: usize = 4 * 2;
pub const LEAF_BYTES: usize = HASH_BYTES + 4 * 2;
pub const VEC_LENGTH: usize = 4;
pub const VEC_STORAGE: usize = 1024 + LEAF_BYTES;
pub const MERKLE_TREE_SPACE: usize = INITIALIZED_BYTES + ROOT_HASH_BYTES + CHILD_BYTES + LEAF_BYTES + VEC_STORAGE;

#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub struct MerkleTree {
    pub is_initialized: bool,
    pub root: Hash,
    pub child: [u32; 2],
    pub leafs: Vec<Leaf>,
}

impl MerkleTree {
    pub fn add_leaf(&mut self, data: &[u8]) {
        let mut hasher = Hasher::default();
        hasher.hash(data);
        let hash = hasher.result();
        let leaf = Leaf {
            root: hash,
            child: [0, 0],
        };
        self.leafs.push(leaf);
        let total = self.leafs.len();
        let mut index = total / 2;
        let number_in_pair = index % 2;
        if number_in_pair == 0 {
            index = index -1;
        }

    }
}

#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub struct Leaf {
    pub root: Hash,
    pub child: [u32; 2],
}

#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub struct InputData {
    //todo
    pub parent: Pubkey,
    pub data: [u8; 32]
}

// Declare and export the program's entrypoint
entrypoint!(process_instruction);

// Accounts required
/// 1. [signer] Payer
/// 2. [writable] MerkleTree account
/// 3.  [] System Program
/// ... [writable] Parent MerkleTree account //todo
pub fn process_instruction(
    program_id: &Pubkey, // Public key of the account the hello world program was loaded into
    accounts: &[AccountInfo], // The account to say hello to
    instruction_data: &[u8], // Ignored, all helloworld instructions are hellos
) -> ProgramResult {

    // Iterating accounts is safer than indexing
    let accounts_iter = &mut accounts.iter();

    // Get the account to say hello to
    let signer_account = next_account_info(accounts_iter)?;
    let merkle_tree_account = next_account_info(accounts_iter)?;
    let system_program = next_account_info(accounts_iter)?;

    if system_program != solana_program::system_program::ID {
        return Err(ProgramError::InvalidArgument)
    }

    if !signer_account.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }

    let (merkle_pda, merkle_bump) = Pubkey::find_program_address(
        &[PREFIX_PDA, signer_account.key.as_ref()],
        program_id
    );

    if merkle_pda != *merkle_tree_account.key || !merkle_tree_account.is_writable || merkle_tree_account.data_is_empty() {
        return Err(ProgramError::InvalidArgument)
    }

    let rent = Rent::get()?;
    let rent_lamports = rent.minimum_balance(MERKLE_TREE_SPACE);



    let create_merkle_pda_ix = &system_instruction::create_account(
        signer_account.key,
        merkle_tree_account.key,
        rent_lamports,
        MERKLE_TREE_SPACE.try_into().unwrap(),
        program_id
    );

    let signers_seeds: &[&[u8]; 3] = &[
        PREFIX_PDA,
        &signer_account.key.to_bytes(),
        &[*merkle_bump],
    ];

    invoke_signed(
        &create_merkle_pda_ix,
        &[
            signer_account.clone(),
            merkle_tree_account.clone(),
            system_program.clone(),
        ],
        &[signers_seeds],
    )?;

    let mut merkle_pda_state = try_from_slice_unchecked::<MerkleTree>(&merkle_tree_account.data.borrow())?;
    merkle_pda_state.is_initialized = true;
    merkle_pda_state.root = Hash::default();


    let mut hasher = Hasher::default();
    hasher.hash(instruction_data);
    let tx_data_hash = hasher.result();
    msg!("tx_data_hash: {:?}", tx_data_hash);
    Ok(())
}