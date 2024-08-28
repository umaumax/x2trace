#!/usr/bin/env python3

from http.server import HTTPServer, SimpleHTTPRequestHandler, test
import os
import argparse
import sys

global args


class CORSRequestHandler (SimpleHTTPRequestHandler):
    def end_headers(self):
        self.send_header('Access-Control-Allow-Origin', '*')
        SimpleHTTPRequestHandler.end_headers(self)


def main():
    parser = argparse.ArgumentParser(
        formatter_class=argparse.ArgumentDefaultsHelpFormatter)
    parser.add_argument('-p', '--port', default=60080, type=int)
    parser.add_argument('-v', '--verbose', action='store_true')
    parser.add_argument('trace_file', nargs='?', default=None)

    global args
    args, extra_args = parser.parse_known_args()
    if args.trace_file:
        dst_file = './trace-file'
        if not os.path.isfile(args.trace_file):
            sys.exit(f'[ERROR] {args.trace_file} not found.')
        if os.path.abspath(args.trace_file) == os.path.abspath(dst_file):
            sys.exit(f'[ERROR] do not use {args.trace_file}')
        if os.path.islink(dst_file):
            os.unlink(dst_file)
        os.symlink(args.trace_file, dst_file)
        if args.verbose:
            print(f'[INFO] created {dst_file} -> {args.trace_file}')
    test(CORSRequestHandler, HTTPServer, port=args.port)


if __name__ == '__main__':
    main()
