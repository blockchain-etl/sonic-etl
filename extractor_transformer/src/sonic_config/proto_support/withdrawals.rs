use super::super::transformation::{err::TransformationErr, bq::integer::TryIntoInteger};
use super::super::proto_codegen::etl::blocks::block::Withdrawal as ProtoWithdrawal;
use alloy::rpc::types::Withdrawal as AlloyWithdrawal;

impl TryFrom<AlloyWithdrawal> for ProtoWithdrawal {
    type Error = TransformationErr;
    #[inline]
    fn try_from(value: AlloyWithdrawal) -> Result<Self, Self::Error> {
        Ok(ProtoWithdrawal {
            index: match value.index.try_into_integer() {
                Ok(num) => num,
                Err(err) => {
                    return Err(TransformationErr::new(
                        err.to_string(),
                        Some("index".to_string()),
                    ));
                }
            },
            validator_index: match value.validator_index.try_into_integer() {
                Ok(num) => num,
                Err(err) => {
                    return Err(TransformationErr::new(
                        err.to_string(),
                        Some("validator_index".to_string()),
                    ));
                }
            },
            address: format!("{}", value.address.0),
            amount: format!("{}", value.amount),
            amount_lossless: format!("{}", value.amount),
        })
    }
}
