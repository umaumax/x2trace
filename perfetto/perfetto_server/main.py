#!/usr/bin/env python3

from http.server import HTTPServer, SimpleHTTPRequestHandler, test
import os
import argparse
import sys
import socket
import psutil
import importlib.resources

COLOR_RED = '\033[31m'
COLOR_GREEN = '\033[32m'
COLOR_RESET = '\033[0m'

global args


def get_all_ip_addresses():
    ip_addresses = []
    for interface, addrs in psutil.net_if_addrs().items():
        if 'br-' in interface or 'docker' in interface:
            continue
        for addr in addrs:
            if addr.family == socket.AF_INET:
                ip_addresses.append((interface, addr.address))
    return ip_addresses


class CORSRequestHandler (SimpleHTTPRequestHandler):
    def do_GET(self):
        if self.path == '/' or self.path == '/index.html':
            target_file = 'index.html'
            if not os.path.exists(target_file):
                print(
                    f'[INFO] load {target_file} file from {__package__} package data')
                index_html_path = importlib.resources.files(
                    __package__).joinpath(target_file)
                with open(index_html_path, 'r', encoding='utf-8') as file:
                    content = file.read()
                self.send_response(200)
                self.send_header('Content-type', 'text/html')
                self.end_headers()
                self.wfile.write(content.encode('utf-8'))
                return
        super(CORSRequestHandler, self).do_GET()

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
    all_ips = get_all_ip_addresses()
    print("[💡Hint]", file=sys.stderr)
    for interface, ip in all_ips:
        url = f"http://{ip}:{args.port}/"
        print(
            f"{COLOR_GREEN}{url:28s}{COLOR_RESET} ({interface})",
            file=sys.stderr)

    test(CORSRequestHandler, HTTPServer, port=args.port)


if __name__ == '__main__':
    main()
