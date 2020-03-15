#[derive(Debug)]
pub enum BankError {
    DepositError,
    WithdrawError,
}

impl std::fmt::Display for BankError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            BankError::DepositError => write!(f, "DepositError"),
            BankError::WithdrawError => write!(f, "WithdrawError"),
        }
    }
}

impl std::error::Error for BankError {}

type Result<T> = std::result::Result<T, BankError>;

pub struct Bank {
    balance: f64,
    uncomplished_deposit: Option<f64>,
}

impl Bank {
    pub fn new(balance: f64) -> Self {
        Bank {
            balance: if balance < 0. { 0. } else { balance },
            uncomplished_deposit: None,
        }
    }

    pub fn balance(&self) -> f64 {
        self.balance
    }

    pub fn pass(&mut self) -> Result<()> {
        self.update(None)
    }

    pub fn deposit(&mut self, amount: f64) -> Result<()> {
        if amount > 0. {
            self.update(Some(amount))
        } else {
            Err(BankError::DepositError)
        }
    }

    pub fn withdraw(&mut self, amount: f64) -> Result<()> {
        if self.balance >= amount {
            self.balance -= amount;
            Ok(())
        } else {
            Err(BankError::WithdrawError)
        }
    }

    fn update(&mut self, deposit_opt: Option<f64>) -> Result<()> {
        if let Some(amount) = std::mem::replace(&mut self.uncomplished_deposit, deposit_opt) {
            self.balance += amount;
        }
        Ok(())
    }
}
