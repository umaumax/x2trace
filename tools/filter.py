#!/usr/bin/env python3
import json
import re
import sys
import argparse
from operator import itemgetter

parser = argparse.ArgumentParser(
    formatter_class=argparse.ArgumentDefaultsHelpFormatter)
parser.add_argument('-b',
                    '--begin-timestamp',
                    help='begin timestamp[ms]',
                    default=0,
                    type=float)
parser.add_argument('-e',
                    '--end-timestamp',
                    help='end timestamp[ms]',
                    default=1000 * 3600,
                    type=float)
parser.add_argument('--include',
                    '--in',
                    help='include function name regex pattern',
                    default='^')
parser.add_argument('--exclude',
                    '--ex',
                    help='exclude function name regex pattern',
                    default='$^')
parser.add_argument('input')
parser.add_argument('args', nargs='*')

args, extra_args = parser.parse_known_args()


def main():
    json_file = open(args.input, 'r')
    json_root = json.load(json_file)

    func_map = {}
    index = 0
    base_timestamp = 0
    for v in json_root:
        name = v['name']
        if name not in func_map:
            func_map[name] = {'stack': [], 'list': []}
        ph = v['ph']
        ts = v['ts']
        if index == 0:
            base_timestamp = ts
        stack = func_map[name]['stack']
        if ph == 'B':
            stack.append((index, ts))
        elif ph == 'E':
            (begin_index, begin_ts) = stack.pop()
            end_index = index
            end_ts = ts
            func_map[name]['list'].append(
                (begin_index, end_index, begin_ts, end_ts))
        elif ph == 'X':
            duration = v['dur']
            begin_ts = ts
            end_ts = ts + duration
            func_map[name]['list'].append(
                (index, -1, begin_ts, end_ts))
        elif ph in ['b', 'i', 'e']:
            pass
        else:
            print("invalid ph:{}".format(ph), file=sys.stderr)
            return 1
        index += 1

    interest_begin_ts = args.begin_timestamp * 1000.0 + base_timestamp
    interest_end_ts = args.end_timestamp * 1000.0 + base_timestamp

    include_pattern = re.compile(args.include)
    exclude_pattern = re.compile(args.exclude)

    valid_index_list = []
    for func_name in func_map.keys():
        result = include_pattern.match(func_name)
        if result is None:
            continue
        result = exclude_pattern.match(func_name)
        if result is not None:
            continue
        for info in func_map[func_name]['list']:
            (begin_index, end_index, begin_ts, end_ts) = info
            if begin_ts < interest_end_ts and interest_begin_ts < end_ts:
                valid_index_list.append(begin_index)
                if end_index >= 0:
                    valid_index_list.append(end_index)

    valid_index_list.sort()

    filtered_json_data = []
    if len(valid_index_list) != 0:
        filtered_json_data = itemgetter(*valid_index_list)(json_root)

    print(json.dumps(list(filtered_json_data)))


if __name__ == "__main__":
    main()
