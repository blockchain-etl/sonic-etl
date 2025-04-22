''' Module to publish ranges '''
from typing import Any, AnyStr, Generator
import os
import time
import sys
import signal
import requests
from google.cloud import pubsub_v1
from pubsub_range_pb2 import IndexingRequest
from dotenv import load_dotenv;

# TODO: support health checks for kubernetes (liveness and readiness)
print("Starting up...")

load_dotenv()


assert os.environ.get("NODE_ADDRESS") is not None
assert os.environ.get("GCP_PROJECT_ID") is not None
assert os.environ.get("NETWORK") is not None
assert os.environ.get("NETWORK") == "mainnet" \
                or os.environ.get("NETWORK") == "testnet"

NODE_ADDRESS = os.environ.get("NODE_ADDRESS")
FALLBACK_NODE_ADDRESS = os.environ.get("FALLBACK_NODE_ADDRESS")
GCP_PROJECT_ID =  os.environ.get("GCP_PROJECT_ID")
NETWORK = os.environ.get("NETWORK")
TIMEOUT = int(os.environ.get("TIMEOUT", "10"))
MAXSIZE = int(os.environ.get("MAXSIZE", "1"))
DEBUG = os.environ.get("DEBUG") == "true"

# the name is what it's called in our GCP project, but the path is what's used by the API
INDEXING_RANGES_TOPIC_NAME = f"indexing-ranges-{NETWORK}"
INDEXING_RANGES_TOPIC_PATH = \
                f"projects/{GCP_PROJECT_ID}/topics/{INDEXING_RANGES_TOPIC_NAME}"
STATEFUL_RESUMPTION_TOPIC_NAME = f"last-indexed-range-{NETWORK}"
STATEFUL_RESUMPTION_TOPIC_PATH = \
            f'projects/{GCP_PROJECT_ID}/topics/{STATEFUL_RESUMPTION_TOPIC_NAME}'

indexing_range_serialized = None

# this publisher will be used for publishing to both the `IndexingRequest` topic
# and the stateful resumption topic `last-indexed-range`
if not DEBUG:
    publisher = pubsub_v1.PublisherClient()

def signal_handler(sig, frame) -> None:
    ''' for handling interrupts (currently SIGINT and SIGTERM) '''
    if sig == signal.SIGINT:
        print('Interrupted by user (SIGINT)')
    elif sig == signal.SIGTERM:
        print('Termination signal received (SIGTERM)')
    else:
        print("Received an unexpected signal. Ignoring...")
        return
    assert indexing_range_serialized is not None
    if not DEBUG:
        future = publisher.publish(topic=STATEFUL_RESUMPTION_TOPIC_PATH, data=indexing_range_serialized)
        print(future.result())
    else:
        print(f"Pushing {indexing_range_serialized} to topic path: {STATEFUL_RESUMPTION_TOPIC_PATH}");
    sys.exit(0)

# Register the signal handlers for SIGINT and SIGTERM
signal.signal(signal.SIGINT, signal_handler)
signal.signal(signal.SIGTERM, signal_handler)

def get_eth_block_number(url: AnyStr) -> int:
    '''Fetch the current Ethereum block number using the JSON-RPC API.'''
    headers = {"Content-Type": "application/json"}
    payload = {
        "jsonrpc": "2.0",
        "method": "eth_blockNumber",
        "params": [],
        "id": 1
    }
    response = requests.post(url, json=payload, headers=headers, timeout=TIMEOUT)
    response.raise_for_status()  # Raises an error for 4xx/5xx responses
    result = response.json()
    if "result" in result:
        # Convert the hex block number to decimal
        return int(result["result"], 16)
    raise ValueError(f"Unexpected response: {result}")

def make_request_with_exponential_backoff(initial_delay:int=1, max_delay:int=60) -> int:
    '''makes an http request. increases the time between requests exponentially,
    until it reaches the `max_delay`. if the FALLBACK_NODE_ADDRESS env var was supplied,
    then requests will alternate between the NODE_ADDRESS and FALLBACK_NODE_ADDRESS'''

    attempt = 0

    if DEBUG:
        return 100

    while True:
        try:
            if FALLBACK_NODE_ADDRESS is not None and attempt % 2 == 1:
                return get_eth_block_number(FALLBACK_NODE_ADDRESS)
            elif NODE_ADDRESS is not None:
                return get_eth_block_number(NODE_ADDRESS)
            else:
                raise ValueError("Missing both FALLBACK_NODE_ADDRES & NODE_ADDRESS")
        except requests.RequestException as e:
            print(e)
            wait_time = min(max_delay, initial_delay * 2 ** attempt)
            attempt += 1
            print(f"Waiting {wait_time} seconds before retrying...")
            time.sleep(wait_time)

def range_generator(start: int, end: int, step: int=999) -> Generator[range, Any, None]:
    '''Divides a range by steps of 1000 values each. Yields values
    using lazy evaluation'''
    current = start
    while current < end:
        # Calculate the next position but ensure it does not exceed 'end'
        next_position = min(current + step - 1, end - 1)
        print("yielding range:", current, next_position + 1)
        yield range(current, next_position + 1)
        # Advance for the next iteration, ensuring disjoint ranges
        current = next_position + 1

def chunk_range(start: int, end: int, chunk_size:int=1000) -> Generator[tuple[int, int], None, None]:
    ''' Yields chunk_size ranges from start to end.  Start and end inclusive,
    meaning all values in [start, end] will be returned.  Returns tuples
    (chunk_start, chunk_end), assume the chunk is inclusive, where the next
    chunk will be start with (chunk_end).  Given a chunk_size of 1, the chunk_start
    and chunk_end will be equal, and each value will be iterated as a pair.'''

    if start > end:
        raise ValueError(f"Start ({start}) cannot be > than End ({end})")
    elif (end - start) < chunk_size:
        yield (start, end)
        return
    elif chunk_size == 1:
        for i in range(start,end+1):
            yield (i,i)
    elif chunk_size <= 0:
        raise ValueError(f"chunk_size must be a positive integer, not {chunk_size}")
    else:
        # Yield each chunk, where the start
        chunk_start = start
        while chunk_start <= end:
            chunk_end = min(chunk_start + chunk_size - 1, end)
            yield (chunk_start, chunk_end)
            chunk_start = chunk_end + 1
        return

def pick_up_from_previous_run() -> int | None:
    ''' Attempts to start from the previous registered '''

    if DEBUG:
        print("In DEBUG mode, returning IndexingRequest(start=5,end=5)")
        return 5;

    last_indexed_range_subscriber = pubsub_v1.SubscriberClient()

    if GCP_PROJECT_ID is None:
        raise ValueError("GCP_PROJECT_ID cannot be None");

    subscription_path = \
        last_indexed_range_subscriber.subscription_path(GCP_PROJECT_ID, \
                                        f"last-indexed-range-{NETWORK}-sub")

    last_indexed_end = None
    while last_indexed_end is None:
        print("Attempting to pull a message from the resumption subscription...")
        # Pull a single message
        response = last_indexed_range_subscriber.pull(
            request={"subscription": subscription_path, "max_messages": 1},
        )

        if response.received_messages:
            received_message = response.received_messages[0]
            indexing_range = IndexingRequest()
            indexing_range.ParseFromString(received_message.message.data)
            last_indexed_end = indexing_range.end
            print(f"Received message: {indexing_range}")
            ack_id = received_message.ack_id
            last_indexed_range_subscriber.acknowledge(request={"subscription": subscription_path, "ack_ids": [ack_id]})
            break
        else:
            print("No messages. Starting from genesis...")
            indexing_range_proto_obj = IndexingRequest(start=0,
                                                       end=0,
                                                       blocks=True,
                                                       logs=True,
                                                       transactions=True,
                                                       receipts=True,
                                                       decoded_events=True,
                                                       traces=True) # fun fact: positional arguments don't work with protobuf initializers
            indexing_range_serialized = indexing_range_proto_obj.SerializeToString()
            assert(indexing_range_serialized is not None)

            if not DEBUG:
                future = publisher.publish(topic=INDEXING_RANGES_TOPIC_PATH, data=indexing_range_serialized)
                print(future.result())
            else:
                print(f"Posting [{indexing_range_proto_obj.start}, {indexing_range_proto_obj.end}] ({indexing_range_serialized}) to topic path: {INDEXING_RANGES_TOPIC_PATH}")
            last_indexed_end = 0


    last_indexed_range_subscriber.close()
    return last_indexed_end

def get_ledger_version() -> int:
    ''' retrieves the current `ledger_version` from the node's REST API '''
    return make_request_with_exponential_backoff()


last_indexed_end = pick_up_from_previous_run()
assert last_indexed_end is not None

# continually requests the maximum available transaction version from the node
while True:
    # wait 1 second between requests (don't want to overload the node with requests, or else it could desync)
    time.sleep(5)
    ledger_version = get_ledger_version()

    if last_indexed_end == ledger_version:
        print(f"No new ledger_version ({ledger_version})")
        continue
    elif last_indexed_end > ledger_version:
        print(f"WARNING: Ledger appeared to revert ({last_indexed_end} -> {ledger_version})")
    else:
        print(f"New ledger_version ({last_indexed_end} -> {ledger_version})", ledger_version)

    # construct indexing ranges from the ledger version, then serialize, and finally publish.
    # NOTE: we use `+ 1` as the next starting value because it is end incl, and we don't want to reindex
    for (start, end) in chunk_range(last_indexed_end+1, ledger_version, chunk_size=1000):
        indexing_range_proto_obj = IndexingRequest(start=start,
                                                   end=end,
                                                   blocks=True,
                                                   logs=True,
                                                   transactions=True,
                                                   receipts=True,
                                                   decoded_events=True,
                                                   traces=True) # fun fact: positional arguments don't work with protobuf initializers
        indexing_range_serialized = indexing_range_proto_obj.SerializeToString()
        assert(indexing_range_serialized is not None)

        if not DEBUG:
            future = publisher.publish(topic=INDEXING_RANGES_TOPIC_PATH, data=indexing_range_serialized)
            print(future.result())
        else:
            print(f"Posting [{indexing_range_proto_obj.start}, {indexing_range_proto_obj.end}] ({indexing_range_serialized}) to topic path: {INDEXING_RANGES_TOPIC_PATH}")

    # advance the range for the next iteration
    last_indexed_end = ledger_version
