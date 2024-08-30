#!/usr/bin/env python3

import csv
import json
import argparse
import sys
from collections import defaultdict
from io import StringIO

import pandas as pd

# autopep8: off
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
# autopep8: on


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

    pid_to_info = None
    df = None
    if args.pid_comm_cmdline:
        # TODO: impl codes to load csv
        df = pd.read_csv(args.pid_comm_cmdline)
        df.set_index('pid', inplace=True)
        # pandas set nan at "", so fill nan as ""
        df = df.fillna("")

        def pid_to_info(pid):
            if pid in df.index:
                comm_value = df.loc[pid, 'comm']
                cmdline_value = df.loc[pid, 'cmdline']
                return (comm_value, cmdline_value)
            else:
                return (None, None)

    trace_events = []
    trace_event_map = defaultdict(list)
    pid_to_comm = {}
    input_file = args.infile
    if args.example:
        input_file = StringIO(jsonl_data.strip())
    # NOTE: for json format
    # with input_file as f:
        # for line in f.readlines():
        # event = json.loads(line.strip())
    # NOTE: for csv format
    dtype_dict = {
        'ts': 'int64',
        'dur': 'int64',
        'cpu': 'int64',
        'pid': 'int64',
        'comm': 'str',
    }
    with pd.read_csv(input_file, dtype=dtype_dict, chunksize=10000) as reader:
        for chunk in reader:
            chunk = chunk.fillna("")
            print(f'✅️[INFO] loaded data [{len(chunk)}]', file=sys.stderr)
            for index, event in chunk.iterrows():
                cpu = event['cpu']
                if 'pid' in event:
                    pid = event['pid']
                    if not isinstance(event['dur'], (int, float)) or not isinstance(
                            cpu, (int, float)) or not isinstance(pid, (int, float)):
                        print(
                            f'🔥[WARN] broken data at L{index+1}\n{event}',
                            file=sys.stderr)
                        continue
                    ts = int(event['ts']) / 1000
                    dur = int(event['dur']) / 1000
                    comm = event['comm'].strip()
                    if comm:
                        pid_to_comm[pid] = comm
                    else:
                        if pid in pid_to_comm:
                            comm = pid_to_comm[pid]
                        else:
                            comm = 'Unknown'
                    name = "{}[{}]".format(comm, pid)
                    trace_events.append({
                        "name": name,
                        "ph": "X",
                        # "pid": 0,
                        "tid": "CPU {}".format(cpu),
                        "ts": ts,
                        "dur": dur,
                    })
                    continue

                event['prev_comm'] = event['comm']
                event['next_comm'] = event['comm']

                # Begin event
                if event['next_pid'] != "":
                    event['next_pid'] = int(event['next_pid'])
                    name = "{}[{}]".format(
                        event['next_comm'].strip(), event['next_pid'])
                    trace_event_map[cpu].append({
                        "name": name,
                        "ph": "B",
                        # "pid": 0,
                        "tid": "CPU {}".format(cpu),
                        # ts is usually in microseconds for Chrome trace
                        "ts": event['ts'] / 1000,
                        # "dur": 0,
                        # "cat": "",
                    })

                # End event
                if event['prev_pid'] != "":
                    event['prev_pid'] = int(event['prev_pid'])
                    # pid = event['pid']
                    tid = event['prev_pid']
                    event_args = {
                        # "pid": pid,
                        "tid": tid,
                        "comm": event['prev_comm'].strip()
                    }
                    if pid_to_info:
                        comm, cmdline = pid_to_info(event['prev_pid'])
                        if comm and cmdline:
                            event_args['comm'] = comm
                            event_args['cmdline'] = cmdline
                            print(
                                f"[INFO] {comm:<15s}[{event['pid']:<7d}] {cmdline}",
                                file=sys.stderr)

                    name = "{}[{}]".format(event['prev_comm'].strip(), tid)
                    if len(
                            trace_event_map[cpu]) > 0 and trace_event_map[cpu][-1]['name'] == name:
                        begin_trace_event = trace_event_map[cpu].pop()
                        trace_events.append({
                            "name": name,
                            "ph": "X",
                            # "pid": 0,
                            "tid": "CPU {}".format(cpu),
                            # ts is usually in microseconds for Chrome trace
                            "ts": begin_trace_event['ts'],
                            "dur": event['ts'] / 1000 - begin_trace_event['ts'],
                            # "dur": 0,
                            # "cat": "",
                            "args": event_args,
                        })
                    else:
                        print(
                            "[WARN] not found begin trace of {}".format(name),
                            file=sys.stderr)

    chrome_trace = {
        "traceEvents": trace_events
    }

    for cpu, events in trace_event_map.items():
        print(
            "[WARN] {} incompleted traces {}".format(cpu, events),
            file=sys.stderr)

    with open(args.output_filepath, "w") as f:
        json.dump(chrome_trace, f)


if __name__ == '__main__':
    main()
