#!/usr/bin/env python3

import sys
import dataclasses
from typing import Dict

import streamlit as st
import plotly.graph_objects as go
import numpy as np


@dataclasses.dataclass
class StackLayer:
    key: str
    next: Dict[str, 'StackLayer']
    self_count: int
    recursive_count: int  # Total counts of upper stacks
    max_count: int  # Maximum counts of one stack in upper stacks

    def add_count(self, keys):
        if len(keys) == 0:
            return 0

        first_key = keys[0]
        if len(keys) == 1:
            # WARN: 見やすくするためのダミー処理
            # import random
            # self.self_count += 1 + random.randrange(10, 20, 2)
            self.self_count += 1
            self.recursive_count += 1
            self.max_count = max(self.max_count, self.self_count)
            return self.max_count
        self.recursive_count += 1
        if first_key not in self.next:
            self.next[first_key] = StackLayer(first_key, {}, 0, 0, 0)
        next_stack_layer = self.next[first_key]
        child_max_count = next_stack_layer.add_count(keys[1:])
        self.max_count = max(self.max_count, child_max_count)
        return self.max_count

    def dump(self, indent=0):
        print(f'{"  "*indent}{self.key}:{self.self_count}({self.recursive_count})')
        indent += 1
        for key, value in self.next.items():
            value.dump(indent)

    def gen_icicle_data(self, parent_id='',
                        ids=None, labels=None, parents=None, values=None, threshold_count=0):
        if not parent_id:
            ids = []
            labels = []
            parents = []
            values = []
        if self.max_count < threshold_count:
            return (ids, labels, parents, values)
        label = self.key
        value = self.self_count
        id = f'{parent_id} - {label}'
        ids.append(id)
        labels.append(label)
        parents.append(parent_id)
        values.append(value)
        for key, value in self.next.items():
            value.gen_icicle_data(
                id, ids, labels, parents, values, threshold_count)
        return (ids, labels, parents, values)


def parse_data_from_stream(stream):
    result = []
    current_section = None

    line_no = 0
    for line in stream:
        line_no += 1
        line = line.strip()
        if not line:
            continue

        if line.startswith('ERROR:'):
            print("[WARN] L{} {}".format(line_no, line), file=sys.stderr)
            continue

        if line == 'kstack:':
            current_section = {'type': 'kstack', 'frames': []}
            result.append(current_section)
        elif line == 'ustack:':
            current_section = {'type': 'ustack', 'frames': []}
            result.append(current_section)
        else:
            if current_section:
                current_section['frames'].append(line)
    return result


def process_data(data):
    ustack_base_stack_layer = StackLayer('[ustack]', {}, 0, 0, 0)
    kstack_base_stack_layer = StackLayer('[kstack]', {}, 0, 0, 0)
    for section in data:
        print(f"Section: {section['type']}")
        frames = list(reversed(section['frames']))
        print()
        if section['type'] == 'ustack':
            ustack_base_stack_layer.add_count(frames)
        else:
            kstack_base_stack_layer.add_count(frames)
        for frame in frames:
            print(f"  {frame}")
    ustack_base_stack_layer.dump()
    kstack_base_stack_layer.dump()

    # example data
    ids = ["Sports",
           "North America", "Europe", "Australia", "North America - Football", "Soccer",
           "North America - Rugby", "Europe - Football", "Rugby",
           "Europe - American Football", "Australia - Football", "Association",
           "Australian Rules", "Autstralia - American Football", "Australia - Rugby",
           "Rugby League", "Rugby Union"
           ]
    labels = ["Sports",
              "North<br>America", "Europe", "Australia", "Football", "Soccer", "Rugby",
              "Football", "Rugby", "American<br>Football", "Football", "Association",
              "Australian<br>Rules", "American<br>Football", "Rugby", "Rugby<br>League",
              "Rugby<br>Union"
              ]
    parents = ["",
               "Sports", "Sports", "Sports", "North America", "North America", "North America", "Europe",
               "Europe", "Europe", "Australia", "Australia - Football", "Australia - Football",
               "Australia - Football", "Australia - Football", "Australia - Rugby",
               "Australia - Rugby"
               ]
    # TODO: 一定時間中に発生した回数が客観的に多いのか、少ないのかがわかるようになるとよい
    # TODO: 実際のデータを利用すること
    # TODO: timeline?や分割する時間間隔の調整ができると面白いかもしれない
    # TODO: 表示の閾値をGUI制御できるようにすること
    threshold_count = 10
    user_ids, user_labels, user_parents, user_values = ustack_base_stack_layer.gen_icicle_data(
        threshold_count=threshold_count)
    kernel_ids, kernel_labels, kernel_parents, kernel_values = kstack_base_stack_layer.gen_icicle_data(
        threshold_count=threshold_count)
    user_parents[0] = '[root]'
    kernel_parents[0] = '[root]'
    ids = ['[root]'] + user_ids + kernel_ids
    labels = ['[root]'] + user_labels + kernel_labels
    parents = [''] + user_parents + kernel_parents
    values = [0] + user_values + kernel_values
    fig = go.Figure(go.Icicle(
        ids=ids,
        labels=labels,
        parents=parents,
        values=values,
        marker_colorscale='Bluered',
        tiling=dict(
            orientation='v',
            flip='y'
        )
    ))
    fig.update_layout(
        margin=dict(t=50, l=25, r=25, b=25),
    )

    st.plotly_chart(fig)


st.set_page_config(
    page_title="bpftrace stack dashboard app",
    layout="wide",
)

if __name__ == "__main__":
    with open('bpftrace.log', 'r') as f:
        parsed_data = parse_data_from_stream(f)

    # parsed_data = parse_data_from_stream(sys.stdin)
    process_data(parsed_data)
