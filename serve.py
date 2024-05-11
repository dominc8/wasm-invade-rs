#!/usr/bin/env python3

import http.server
import socketserver

PORT = 8080

Handler = http.server.SimpleHTTPRequestHandler

Handler.extensions_map[".wasm"] = "application/wasm"

keep_running = True

with socketserver.TCPServer(("", PORT), Handler, bind_and_activate=False) as httpd:
    httpd.allow_reuse_address = True
    httpd.allow_reuse_port = True

    httpd.server_bind()
    httpd.server_activate()
    print("serving at port", PORT)

    while keep_running:
        try:
            httpd.handle_request()
        except KeyboardInterrupt:
            keep_running = False;
