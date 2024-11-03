use solana_sdk::{
    instruction::{CompiledInstruction, Instruction},
    pubkey::Pubkey,
};

/// Compile an instruction to a compiled instruction
/// 
/// # Arguments
/// 
/// * `ix` - The instruction to compile
/// * `account_keys` - The account keys to use for the compiled instruction
/// 
/// # Returns
/// 
/// A compiled instruction  
pub fn compile_instruction(ix: &Instruction, account_keys: &[Pubkey]) -> CompiledInstruction {
    // Find program id index
    let program_idx = account_keys
        .iter()
        .position(|key| key == &ix.program_id)
        .unwrap_or_default() as u8;

    // Map account metas to their indices in account_keys
    let accounts: Vec<u8> = ix
        .accounts
        .iter()
        .map(|account| {
            account_keys
                .iter()
                .position(|key| key == &account.pubkey)
                .unwrap() as u8
        })
        .collect();

    CompiledInstruction {
        program_id_index: program_idx,
        accounts,
        data: ix.data.clone(),
    }
}