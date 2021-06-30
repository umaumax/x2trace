#!/usr/bin/env python3
import sys
import json
import argparse

import numpy as np
import rich

parser = argparse.ArgumentParser(
    formatter_class=argparse.ArgumentDefaultsHelpFormatter)
parser.add_argument(
    '--min',
    help='min duration of outlier[ms]',
    default=100,
    type=float)
parser.add_argument(
    '--th',
    help='threshold of outlier',
    default=100.0,
    type=float)
parser.add_argument('input')
parser.add_argument('args', nargs='*')

args, extra_args = parser.parse_known_args()


def range_of_interest_outlier_filter(data):
    return data[data >= args.min]


def find_outliers(data, m=100.):
    d = np.abs(data - np.median(data))
    mdev = np.median(d)
    s = d / mdev if mdev else 0.
    return np.array(data)[s >= m]


def main():
    json_file = open(args.input, 'r')
    json_root = json.load(json_file)

    th = args.th

    func_map = {}
    for v in json_root:
        name = v['name']
        if name not in func_map:
            func_map[name] = {'stack': [], 'durations': []}
        ph = v['ph']
        ts = v['ts']
        stack = func_map[name]['stack']
        if ph == 'B':
            stack.append(ts)
        if ph == 'E':
            duration = ts - stack.pop()
            func_map[name]['durations'].append(duration / 1000.0)

    for func_name in func_map.keys():
        durations = func_map[func_name]['durations']
        if len(durations) > 100:
            outliers = find_outliers(durations, th)
            outliers = range_of_interest_outlier_filter(outliers)
            if len(outliers) > 0:
                median = np.median(durations)
                rich.print(
                    "[bold green]name[/bold green]:[bold magenta]{}[/bold magenta]".format(func_name))
                rich.print(
                    '[bold green]outliers(ms)[/bold green]:{}'.format(outliers))
                rich.print(
                    '[bold green]median(ms)[/bold green]:{}'.format(median))
                print()


if __name__ == "__main__":
    main()
