use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint,
    entrypoint::ProgramResult,
    msg,
    program::{invoke, invoke_signed},
    program_error::ProgramError,
    program_option::COption,
    program_pack::Sealed,
    program_pack::{IsInitialized, Pack},
    pubkey::Pubkey,
    sysvar::{rent::Rent, Sysvar},
};

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Token {
    pub mint: Pubkey,
    pub owner: Pubkey,
    pub amount: u64,
}

impl Sealed for Token {}

impl IsInitialized for Token {
    fn is_initialized(&self) -> bool {
        self.amount > 0
    }
}

impl Pack for Token {
    const LEN: usize = 64;

    fn unpack_from_slice(src: &[u8]) -> Result<Self, ProgramError> {
        let amount = u64::from_le_bytes(src[0..8].try_into().unwrap());
        let mint = Pubkey::new_from_array(src[8..40].try_into().unwrap());
        let owner = Pubkey::new_from_array(src[40..72].try_into().unwrap());

        Ok(Token {
            mint,
            owner,
            amount,
        })
    }

    fn pack_into_slice(&self, dst: &mut [u8]) {
        let amount = self.amount.to_le_bytes();
        let mint = self.mint.to_bytes();
        let owner = self.owner.to_bytes();

        dst[0..8].copy_from_slice(&amount);
        dst[8..40].copy_from_slice(&mint);
        dst[40..72].copy_from_slice(&owner);
    }
}

entrypoint!(process_instruction);

pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    let instruction = instruction_data[0];

    match instruction {
        0 => {
            let amount = u64::from_le_bytes(instruction_data[1..9].try_into().unwrap());
            mint_tokens(program_id, accounts, amount)
        }
        1 => {
            let amount = u64::from_le_bytes(instruction_data[1..9].try_into().unwrap());
            transfer_tokens(program_id, accounts, amount)
        }
        _ => Err(ProgramError::InvalidInstructionData),
    }
}

fn mint_tokens(program_id: &Pubkey, accounts: &[AccountInfo], amount: u64) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let mint_account = next_account_info(account_info_iter)?;
    let token_account = next_account_info(account_info_iter)?;

    if token_account.owner != program_id {
        return Err(ProgramError::IncorrectProgramId);
    }

    let mut token_data = Token::unpack_unchecked(&token_account.try_borrow_data()?)?;
    if !token_data.is_initialized() {
        token_data.mint = *mint_account.key;
        token_data.owner = *mint_account.key;
        token_data.amount = amount;
        Token::pack(token_data, &mut token_account.try_borrow_mut_data()?)?;
    } else {
        return Err(ProgramError::AccountAlreadyInitialized);
    }

    Ok(())
}

fn transfer_tokens(program_id: &Pubkey, accounts: &[AccountInfo], amount: u64) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let source_account = next_account_info(account_info_iter)?;
    let destination_account = next_account_info(account_info_iter)?;
    let authority_account = next_account_info(account_info_iter)?;

    if !authority_account.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }

    let mut source_data = Token::unpack(&source_account.try_borrow_data()?)?;
    let mut destination_data = Token::unpack_unchecked(&destination_account.try_borrow_data()?)?;

    if source_data.amount < amount {
        return Err(ProgramError::InsufficientFunds);
    }

    source_data.amount -= amount;
    destination_data.amount += amount;

    Token::pack(source_data, &mut source_account.try_borrow_mut_data()?)?;
    Token::pack(
        destination_data,
        &mut destination_account.try_borrow_mut_data()?,
    )?;

    Ok(())
}
