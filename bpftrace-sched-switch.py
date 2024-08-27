#!/usr/bin/env python3

import json
import argparse
import sys
from io import StringIO


jsonl_data = """
{"ts":450480781978227,"cpu": 7,"pid":3241856,"prev_pid":3241856,"prev_comm":"bpftrace       ","next_pid":      0,"next_comm":"swapper/7      "}
{"ts":450480781984008,"cpu": 7,"pid":      0,"prev_pid":      0,"prev_comm":"swapper/7      ","next_pid":3241856,"next_comm":"bpftrace       "}
{"ts":450480782014625,"cpu": 6,"pid":      0,"prev_pid":      0,"prev_comm":"swapper/6      ","next_pid":3241857,"next_comm":"tee            "}
{"ts":450480782014946,"cpu": 7,"pid":3241856,"prev_pid":3241856,"prev_comm":"bpftrace       ","next_pid":      0,"next_comm":"swapper/7      "}
{"ts":450480782024503,"cpu":11,"pid":      0,"prev_pid":      0,"prev_comm":"swapper/11     ","next_pid":3190218,"next_comm":"kworker/u32:2  "}
{"ts":450480782025345,"cpu": 7,"pid":      0,"prev_pid":      0,"prev_comm":"swapper/7      ","next_pid":3241856,"next_comm":"bpftrace       "}
{"ts":450480782027339,"cpu":11,"pid":3190218,"prev_pid":3190218,"prev_comm":"kworker/u32:2  ","next_pid":      0,"next_comm":"swapper/11     "}
{"ts":450480782028441,"cpu": 8,"pid":      0,"prev_pid":      0,"prev_comm":"swapper/8      ","next_pid":2985788,"next_comm":"containerd-shim"}
{"ts":450480782030334,"cpu": 6,"pid":3241857,"prev_pid":3241857,"prev_comm":"tee            ","next_pid":      0,"next_comm":"swapper/6      "}
{"ts":450480782032859,"cpu": 8,"pid":2985780,"prev_pid":2985788,"prev_comm":"containerd-shim","next_pid":      0,"next_comm":"swapper/8      "}
"""


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('-o', '--output-filepath', default='/dev/stdout')
    parser.add_argument('--pid-comm-cmdline', default='')
    parser.add_argument('--example', action='store_true')
    parser.add_argument(
        'infile',
        nargs='?',
        type=argparse.FileType(),
        default=sys.stdin)
    args = parser.parse_args()

    if args.pid_comm_cmdline:
        # TODO: impl codes to load csv
        pass

    trace_events = []
    input_file = args.infile
    if args.example:
        input_file = StringIO(jsonl_data.strip())
    with input_file as f:
        for line in f.readlines():
            event = json.loads(line.strip())

            # Begin event
            if event['next_pid'] != 0:
                trace_events.append({
                    "name": event['next_comm'].strip(),
                    "ph": "B",
                    # "pid": 0,
                    "tid": event['cpu'],
                    # ts is usually in microseconds for Chrome trace
                    "ts": event['ts'] / 1000,
                    # "dur": 0,
                    # "cat": "",
                })

            # End event
            if event['prev_pid'] != 0:
                trace_events.append({
                    "name": event['prev_comm'].strip(),
                    "ph": "E",
                    # "pid": 0,
                    "tid": event['cpu'],
                    # ts is usually in microseconds for Chrome trace
                    "ts": event['ts'] / 1000,
                    # "dur": 0,
                    # "cat": "",
                    "args": {
                        "pid": event['pid'],
                        "tid": event['prev_pid'],
                        "comm": event['prev_comm'].strip()
                    }
                })

    chrome_trace = {
        "traceEvents": trace_events
    }

    with open(args.output_filepath, "w") as f:
        json.dump(chrome_trace, f)


if __name__ == '__main__':
    main()
