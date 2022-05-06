#!/usr/bin/env python3

import argparse
import json
import re
import sys


def remove_prefix(text, prefix):
    if text.startswith(prefix):
        return text[len(prefix):]
    return text


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('-v', '--verbose', action='store_true')
    parser.add_argument('input')
    parser.add_argument('args', nargs='*')

    trace_list = []
    args, extra_args = parser.parse_known_args()
    task_map = {}

    # input file example
    '''
               time    cpu  task name                       wait time  sch delay   run time
                            [tid/pid]                          (msec)     (msec)     (msec)
    --------------- ------  ------------------------------  ---------  ---------  ---------
     4282978.085809 [0004]  <idle>                              0.000      0.000      0.000
     4282978.125710 [0004]  fzf[8489/4922]                      0.000      0.002      0.010
     4282978.223179 [0009]  tmux: server[27077]                 0.000      0.002      1.462
    '''
    with open(args.input) as file:
        for line in file:
            cols = line.split()
            # skip header lines
            if cols[0] == 'time' or cols[0] == '[tid/pid]' or re.search(
                    '^-+$', cols[0]):
                continue
            # e.g. 1001.610442 lost 78106 events on cpu 0
            if cols[1] == 'lost':
                print("skip broken line: {}".format(line.rstrip()), file=sys.stderr)
                continue

            timestamp = float(cols[0]) * 1000.0 * 1000.0  # sec to us
            cpu = int(cols[1].lstrip('[').rstrip(']'))

            task_name = ' '.join(cols[2:-3])  # task name may have spaces
            if task_name == '<idle>':
                continue

            wait_time = float(cols[-3]) * 1000.0  # ms to us
            sch_delay = float(cols[-2]) * 1000.0  # ms to us
            run_time = float(cols[-1]) * 1000.0  # ms to us

            ret = re.search(r'(?P<command>[^[]+)\[(?P<pid>(?P<tid>[0-9]+))\]',
                            task_name)
            if ret is not None:
                command = ret.group("command")
                pid = int(ret.group("pid"))
                tid = int(ret.group("tid"))
            else:
                ret = re.search(r'(?P<command>[^[]+)\[(?P<tid>[0-9]+)/(?P<pid>[0-9]+)\]',
                                task_name)
                if ret is not None:
                    command = ret.group("command")
                    pid = int(ret.group("pid"))
                    tid = int(ret.group("tid"))

            duration = run_time

            pid = 'CPU'
            cpu = 'CPU:' + str(cpu)
            # slice
            trace_list += [{
                "name": "sch_delay:{}({})".format(command, tid),
                "cat": "{}".format(command),
                "ph": 'X',
                "ts": timestamp - duration - sch_delay,
                "dur": sch_delay,
                "pid": pid,
                "tid": cpu,
                "args": {}
            }]
            trace_list += [{
                "name": "{}({})".format(command, tid),
                "cat": "{}".format(command),
                "ph": 'X',
                "ts": timestamp - duration,
                "dur": duration,
                "pid": pid,
                "tid": cpu,
                "args": {}
            }]

            # flow events
            ph = 's'
            if task_name in task_map:
                ph = 't'
            else:
                task_map[task_name] = True
            trace_list += [{
                "name": "{}({})".format(command, tid),
                "cat": "{}".format(command),
                "ph": ph,
                "ts": timestamp - duration,
                "dur": duration,
                "pid": pid,
                "tid": cpu,
                "id": tid,
                "args": {}
            }]

    print(json.dumps(list(trace_list)))


if __name__ == '__main__':
    main()
