#!/usr/bin/env python3

import http.server
import socketserver

PORT = 8080

Handler = http.server.SimpleHTTPRequestHandler

Handler.extensions_map[".wasm"] = "application/wasm"

keep_running = True

with socketserver.TCPServer(("", PORT), Handler) as httpd:
    print("serving at port", PORT)

    while keep_running:
        try:
            httpd.handle_request()
        except KeyboardInterrupt:
            keep_running = False;
