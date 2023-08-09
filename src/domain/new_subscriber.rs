use crate::domain::{ValidEmail, ValidName};

pub struct NewSubscriber {
    pub email: ValidEmail,
    pub name: ValidName,
}
