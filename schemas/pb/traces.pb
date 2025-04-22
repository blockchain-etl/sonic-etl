
�
traces.proto
etl.traces"�
Trace

block_hash (	R	blockHash!
block_number (RblockNumber'
block_timestamp (RblockTimestamp)
transaction_hash (	RtransactionHash+
transaction_index (RtransactionIndex

trace_type (	R	traceType#

trace_address (RtraceAddress%
subtrace_count (R
subtraceCount5
action	 (2.etl.traces.Trace.TraceActionRaction5
result
 (2.etl.traces.Trace.TraceResultRresult
error (	Rerror
trace_index (R
traceIndex�
TraceAction!
from_address (	RfromAddress

to_address (	R	toAddress
	call_type (	RcallType
gas (Rgas
input (	Rinput
value (	Rvalue%
value_lossless (	R
valueLossless
init (	Rinit
author	 (	Rauthor
reward_type
 (	R
rewardType%
refund_address (	R
refundAddress%
refund_balance (	R
refundBalance6
refund_balance_lossless
 (	RrefundBalanceLossless6
self_destructed_address (	RselfDestructedAddressn
TraceResult
gas_used (RgasUsed
output (	Routput
address (	Raddress
code (	Rcode