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
    parser.add_argument(
        '--timestamp-offset',
        type=float,
        default=0.0,
        help='unit is sec')
    parser.add_argument(
        '-f',
        '--format',
        default='auto',
        help='auto, timehist, normal')
    parser.add_argument(
        '-o',
        '--output',
        default='/dev/stdout')
    parser.add_argument('input')
    parser.add_argument('args', nargs='*')

    trace_list = []
    args, extra_args = parser.parse_known_args()
    task_map = {}

    irq_id_name_dict = {}
    # input file example
    '''
               time    cpu  task name                       wait time  sch delay   run time
                            [tid/pid]                          (msec)     (msec)     (msec)
    --------------- ------  ------------------------------  ---------  ---------  ---------
     4282978.085809 [0004]  <idle>                              0.000      0.000      0.000
     4282978.125710 [0004]  fzf[8489/4922]                      0.000      0.002      0.010
     4282978.223179 [0009]  tmux: server[27077]                 0.000      0.002      1.462
    '''
    format = args.format
    with open(args.input) as file:
        for line in file:
            cols = line.split()

            if format == 'auto':
                if cols[0] == 'time' and cols[1] == 'cpu':
                    format = 'timehist'
                else:
                    format = 'normal'

            if format == 'normal':
                # irq events
                '''
                e.g.
                    swapper     0 [002]  5610.1315104357: irq:irq_handler_entry: irq=17 name=twd
                '''
                ret = re.search(
                    r'^ *(?P<command>.+) +(?P<tid>[0-9]+) +\[(?P<cpu>[0-9]+)\] +(?P<timestamp>[0-9]+\.[0-9]+): +(?P<event_name>[^:]+:[^:]+): (?P<event_args>.+)$',
                    line)
                if ret is None:
                    print(
                        "skip broken line: {}".format(
                            line.rstrip()),
                        file=sys.stderr)
                    continue

                task_name = ret.group("command")
                tid = int(ret.group("tid"))
                cpu = int(ret.group("cpu"))
                timestamp = float(ret.group("timestamp")) * \
                    1000.0 * 1000.0  # sec to us
                timestamp += args.timestamp_offset
                event_name = ret.group("event_name")
                event_args = ret.group("event_args")

                ph = ''
                line_parsed_flag = False
                if event_name == 'irq:irq_handler_entry':
                    ret = re.search(
                        r'irq=(?P<irq>[0-9]+) name=(?P<name>.+)', event_args)
                    if ret is not None:
                        irq_id = int(ret.group("irq"))
                        irq_name = ret.group("name")
                        ph = 'B'
                        irq_id_name_dict[irq_id] = irq_name
                        name = "irq_handler: {}({})".format(irq_name, irq_id)
                        line_parsed_flag = True
                elif event_name == 'irq:irq_handler_exit':
                    ret = re.search(
                        r'irq=(?P<irq>[0-9]+) ret=(?P<ret>.+)', event_args)
                    if ret is not None:
                        irq_id = int(ret.group("irq"))
                        irq_name = irq_id_name_dict[irq_id]
                        irq_ret = ret.group("ret")
                        name = "irq_handler: {}({})".format(irq_name, irq_id)
                        ph = 'E'
                        line_parsed_flag = True
                elif event_name in ['irq:softirq_raise', 'irq:softirq_entry', 'irq:softirq_exit']:
                    ret = re.search(
                        r'vec=(?P<vec>[0-9]+) \[action=(?P<action>.+)\]', event_args)
                    if ret is not None:
                        soft_irq_id = int(ret.group("vec"))
                        soft_irq_action = ret.group("action")
                        if event_name == 'irq:softirq_raise':
                            # temporary ignored
                            continue
                        if event_name == 'irq:softirq_entry':
                            ph = 'B'
                        if event_name == 'irq:softirq_exit':
                            ph = 'E'
                        name = "softirq: {}({})".format(
                            soft_irq_action, soft_irq_id)
                        line_parsed_flag = True
                elif event_name == 'sched:sched_switch':
                    # temporary skipped
                    line_parsed_flag = True
                    continue
                else:
                    print("unknown event name '{}' at '{}'".format(
                        event_name, line.rstrip()), file=sys.stderr)
                    break

                if not line_parsed_flag:
                    print(
                        "skip broken line: {}".format(
                            line.rstrip()),
                        file=sys.stderr)
                    break

                trace_list += [{
                    "name": name,
                    "cat": 'irq',
                    "ph": ph,
                    "ts": timestamp,
                    # "pid": 'irq',
                    # "tid": cpu,
                    "pid": cpu,
                    "tid": name,
                    "args": {}
                }]

            if format == 'timehist':
                # skip header lines
                if cols[0] == 'time' or cols[0] == '[tid/pid]' or re.search(
                        '^-+$', cols[0]):
                    continue
                # e.g. 1001.610442 lost 78106 events on cpu 0
                if cols[1] == 'lost':
                    print(
                        "skip broken line: {}".format(
                            line.rstrip()),
                        file=sys.stderr)
                    continue

                timestamp = float(cols[0]) * 1000.0 * 1000.0  # sec to us
                timestamp += args.timestamp_offset
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

                cpu_pid = 'CPU'
                cpu_tid = 'CPU:' + str(cpu)

                duration = run_time
                start_timestamp = timestamp - duration

                # only show long sch_delay slice(unit is us)
                if sch_delay > 10:
                    sch_delay_timestamp = timestamp - duration - sch_delay
                    # slice
                    trace_list += [{
                        "name": "sch_delay:{}({})".format(command, tid),
                        "cat": "{}".format(command),
                        "ph": 'X',
                        "ts": sch_delay_timestamp,
                        "dur": sch_delay,
                        "pid": cpu_pid,
                        "tid": cpu_tid,
                        "args": {}
                    }]
                    trace_list += [{
                        "name": "sch_delay:{}({})".format(command, tid),
                        "cat": "{}".format(command),
                        "ph": 'X',
                        "ts": sch_delay_timestamp,
                        "dur": sch_delay,
                        "pid": "{}({})".format(command, pid),
                        "tid": tid,
                        "args": {}
                    }]
                    print(
                        "long sch_delay duration at: {}({}) {}us".format(
                            command, tid, sch_delay), file=sys.stderr)
                # slice
                trace_list += [{
                    "name": "{}({})".format(command, tid),
                    "cat": "{}".format(command),
                    "ph": 'X',
                    "ts": start_timestamp,
                    "dur": duration,
                    "pid": cpu_pid,
                    "tid": cpu_tid,
                    "args": {}
                }]
                trace_list += [{
                    "name": "{}({})".format(command, tid),
                    "cat": "{}".format(command),
                    "ph": 'X',
                    "ts": start_timestamp,
                    "dur": duration,
                    "pid": "{}({})".format(command, pid),
                    "tid": tid,
                    "args": {}
                }]

                # flow events
                ph = 's'
                if task_name in task_map:
                    ph = 't'
                    task_map[task_name]['cnt'] += 1
                    task_map[task_name]['duration'] += duration
                else:
                    task_map[task_name] = {'duration': duration, 'cnt': 1}
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

    with open(args.output, mode='w') as f:
        f.write(json.dumps(list(trace_list)))

    # print statistics of timehist result
    if len(task_map) > 0:
        duration_sum = 0.0
        cnt_sum = 0
        print(
            "{:>24} {:>9}   {:>6}".format(
                "name",
                "duration",
                "cnt"),
            file=sys.stderr)
        for k, v in reversed(sorted(task_map.items(),
                                    key=lambda item: item[1]['duration'])):
            duration = v['duration']
            cnt = v['cnt']
            print(
                "{:>24} {:>9.3f}ms {:>6}".format(
                    k,
                    duration /
                    1000.0,
                    cnt),
                file=sys.stderr)
            duration_sum += duration
            cnt_sum += cnt
        print(
            "{:>24} {:>9.3f}ms {:>6}".format(
                "total",
                duration_sum /
                1000.0,
                cnt_sum),
            file=sys.stderr)


if __name__ == '__main__':
    main()
