use super::{
    generic::{DynEventCatalog, EventCatalog, GetEventBySigErr, LogDecodeErr},
    mapped::EventMapCatalog,
};
use alloy::{json_abi::Event, primitives::B256, rpc::types::Log};

/// [ErcEventCatalog] handles all Ethereum's
#[derive(Debug, Clone)]
pub struct ErcEventCatalog {
    eventmap: EventMapCatalog,
}

impl Default for ErcEventCatalog {
    #[inline]
    fn default() -> Self {
        // Create the events
        let erc20_transform =
            Event::parse("Transfer(address indexed from, address indexed to, uint256 value)")
                .expect("Failed with erc20_transform");
        let erc20_approval =
            Event::parse("Approval(address indexed owner, address indexed spender, uint256 value)")
                .expect("Failed with erc20_approval");
        let erc223_transfer = Event::parse(
            "Transfer(address indexed from, address indexed to, uint256 value, bytes data)",
        )
        .expect("Failed with erc223_transfer");
        let erc721_transfer = Event::parse(
            "Transfer(address indexed from, address indexed to, uint256 indexed tokenId)",
        )
        .expect("Failed with erc721_transfer");
        let erc721_approval = Event::parse(
            "Approval(address indexed owner, address indexed approved, uint256 indexed tokenId)",
        )
        .expect("Failed with erc721_approval");
        let erc721_approval_for_all = Event::parse(
            "ApprovalForAll(address indexed owner, address indexed operator, bool approved)",
        )
        .expect("Failed with erc721_approval_for_all");
        let erc777_sent = Event::parse("Sent(address indexed operator, address indexed from, address indexed to, uint256 amount, bytes data, bytes operatorData)").expect("Failed with erc777_sent");
        let erc777_minted = Event::parse("Minted(address indexed operator, address indexed to, uint256 amount, bytes data, bytes operatorData)").expect("Failed with erc777_minted");
        let erc777_burned = Event::parse("Burned(address indexed operator, address indexed from, uint256 amount, bytes data, bytes operatorData)").expect("Failed with erc777_burned");
        let erc777_authorized_operator = Event::parse(
            "AuthorizedOperator(address indexed operator, address indexed tokenHolder)",
        )
        .expect("Failed with erc777_authorized_operator");
        let erc777_revoked_operator =
            Event::parse("RevokedOperator(address indexed operator, address indexed tokenHolder)")
                .expect("Failed with erc777_revoked_operator");
        let erc1155_transfer_single = Event::parse("TransferSingle(address indexed operator, address indexed from, address indexed to, uint256 id, uint256 value)").expect("Failed with erc1155_transfer_single");
        let erc1155_transfer_batch = Event::parse("TransferBatch(address indexed operator, address indexed from, address indexed to, uint256[] ids, uint256[] values)").expect("Failed with erc1155_transfer_batch");
        let erc1155_approval_for_all = Event::parse(
            "ApprovalForAll(address indexed account, address indexed operator, bool approved)",
        )
        .expect("Failed with erc1155_approval_for_all");
        let erc1155_uri =
            Event::parse("URI(string value, uint256 indexed id)").expect("Failed with erc1155_uri");
        let erc1400_transfer_with_data = Event::parse(
            "TransferWithData(address indexed from, address indexed to, uint256 value, bytes data)",
        )
        .expect("Failed with erc1400_transfer_with_data");
        let erc1400_transfer_by_partition = Event::parse("TransferByPartition(bytes32 indexed partition, address indexed from, address indexed to, uint256 value, bytes data, bytes operatorData)").expect("Failed with erc1400_transfer_by_partition");
        let erc1400_authorized_operator = Event::parse(
            "AuthorizedOperator(address indexed operator, address indexed tokenHolder)",
        )
        .expect("Failed with erc1400_authorized_operator");
        let erc1400_revoked_operator =
            Event::parse("RevokedOperator(address indexed operator, address indexed tokenHolder)")
                .expect("Failed with erc1400_revoked_operator");
        let erc2612_approval =
            Event::parse("Approval(address indexed owner, address indexed spender, uint256 value)")
                .expect("Failed with erc2612_approval");
        let erc4626_deposit = Event::parse("Deposit(address indexed sender, address indexed owner, uint256 assets, uint256 shares)").expect("Failed with erc4626_deposit");
        let erc4626_withdraw = Event::parse("Withdraw(address indexed sender, address indexed receiver, address indexed owner, uint256 assets, uint256 shares)").expect("Failed with erc4626_withdraw");

        // SONIC SPECIFIC
        let sfc_createdvalidator = Event::parse("event CreatedValidator(uint256 indexed validatorID,address indexed administrator,uint256 createdEpoch,uint256 createdTime)").expect("Failed with sfc_createdvalidator");
        let sfc_deactivatedvalidator = Event::parse("event DeactivatedValidator(uint256 indexed validatorID, uint256 deactivatedEpoch, uint256 deactivatedTime)").expect("Failed with sfc_deactivatedvalidator");
        let sfc_delegated = Event::parse("event Delegated(address indexed delegator, uint256 indexed toValidatorID, uint256 amount)").expect("Failed with sfc_delegated");
        let sfc_undelegated = Event::parse("event Undelegated(address indexed delegator, uint256 indexed toValidatorID, uint256 indexed requestID, uint256 amount)").expect("Failed with sfc_undelegated");
        let sfc_withdrawn = Event::parse("event Withdrawn(address indexed delegator, uint256 indexed toValidatorID, uint256 indexed requestID, uint256 amount, uint256 penalty)").expect("Failed with sfc_withdrawn");
        let sfc_claimedrewards = Event::parse("event ClaimedRewards(address indexed delegator, uint256 indexed toValidatorID, uint256 rewards)").expect("Failed with sfc_claimedrewards");
        let sfc_restakedrewards = Event::parse("event RestakedRewards(address indexed delegator, uint256 indexed toValidatorID, uint256 rewards)").expect("Failed with sfc_restakedrewards");


        // Create the event map
        let mut eventmap = EventMapCatalog::with_capacity(22);
        // Add each event
        eventmap
            .add_event(&erc20_transform)
            .expect("Failed to add erc20_transform to EventMapCatalog");
        eventmap
            .add_event(&erc20_approval)
            .expect("Failed to add erc20_approval to EventMapCatalog");
        eventmap
            .add_event(&erc223_transfer)
            .expect("Failed to add erc223_transfer to EventMapCatalog");
        eventmap
            .add_event(&erc721_transfer)
            .expect("Failed to add erc721_transfer to EventMapCatalog");
        eventmap
            .add_event(&erc721_approval)
            .expect("Failed to add erc721_approval to EventMapCatalog");
        eventmap
            .add_event(&erc721_approval_for_all)
            .expect("Failed to add erc721_approval_for_all to EventMapCatalog");
        eventmap
            .add_event(&erc777_sent)
            .expect("Failed to add erc777_sent to EventMapCatalog");
        eventmap
            .add_event(&erc777_minted)
            .expect("Failed to add erc777_minted to EventMapCatalog");
        eventmap
            .add_event(&erc777_burned)
            .expect("Failed to add erc777_burned to EventMapCatalog");
        eventmap
            .add_event(&erc777_authorized_operator)
            .expect("Failed to add erc777_authorized_operator to EventMapCatalog");
        eventmap
            .add_event(&erc777_revoked_operator)
            .expect("Failed to add erc777_revoked_operator to EventMapCatalog");
        eventmap
            .add_event(&erc1155_transfer_single)
            .expect("Failed to add erc1155_transfer_single to EventMapCatalog");
        eventmap
            .add_event(&erc1155_transfer_batch)
            .expect("Failed to add erc1155_transfer_batch to EventMapCatalog");
        eventmap
            .add_event(&erc1155_approval_for_all)
            .expect("Failed to add erc1155_approval_for_all to EventMapCatalog");
        eventmap
            .add_event(&erc1155_uri)
            .expect("Failed to add erc1155_uri to EventMapCatalog");
        eventmap
            .add_event(&erc1400_transfer_with_data)
            .expect("Failed to add erc1400_transfer_with_data to EventMapCatalog");
        eventmap
            .add_event(&erc1400_transfer_by_partition)
            .expect("Failed to add erc1400_transfer_by_partition to EventMapCatalog");
        eventmap
            .add_event(&erc1400_authorized_operator)
            .expect("Failed to add erc1400_authorized_operator to EventMapCatalog");
        eventmap
            .add_event(&erc1400_revoked_operator)
            .expect("Failed to add erc1400_revoked_operator to EventMapCatalog");
        eventmap
            .add_event(&erc2612_approval)
            .expect("Failed to add erc2612_approval to EventMapCatalog");
        eventmap
            .add_event(&erc4626_deposit)
            .expect("Failed to add erc4626_deposit to EventMapCatalog");
        eventmap
            .add_event(&erc4626_withdraw)
            .expect("Failed to add erc4626_withdraw to EventMapCatalog");
        eventmap
            .add_event(&sfc_createdvalidator)
            .expect("Failed to add sfc_createdvalidator to EventMapCatalog");
        eventmap
            .add_event(&sfc_deactivatedvalidator)
            .expect("Failed to add sfc_deactivatedvalidator to EventMapCatalog");
        eventmap
            .add_event(&sfc_delegated)
            .expect("Failed to add sfc_delegated to EventMapCatalog");
        eventmap
            .add_event(&sfc_undelegated)
            .expect("Failed to add sfc_undelegated to EventMapCatalog");
        eventmap
            .add_event(&sfc_withdrawn)
            .expect("Failed to add sfc_withdrawn to EventMapCatalog");
        eventmap
            .add_event(&sfc_claimedrewards)
            .expect("Failed to add sfc_claimedrewards to EventMapCatalog");
        eventmap
            .add_event(&sfc_restakedrewards)
            .expect("Failed to add sfc_restakedrewards to EventMapCatalog");


        // Return the wrapped mapping
        Self { eventmap }
    }
}

impl ErcEventCatalog {
    #[inline]
    pub fn new() -> Self {
        Self::default()
    }
}

impl EventCatalog for ErcEventCatalog {
    fn get_event_by_signature_and_ntopics(
        &self,
        signature: &B256,
        n_topics: u8,
    ) -> Result<&Event, GetEventBySigErr> {
        self.eventmap
            .get_event_by_signature_and_ntopics(signature, n_topics)
    }

    fn attempt_decode_log(
        &self,
        log: &Log,
    ) -> Result<super::generic::DecodedEventExt, LogDecodeErr> {
        self.eventmap.attempt_decode_log(log)
    }
}
