package main

// Single-binary configuration: one gateway serves ESP32 + STM32F4 +
// Pico/RP2350 boards at the same time ("when the gateway is running I can
// use all three boards with all features").
//
// Ports (defaults; env-overridable so three legacy instances can still run
// side by side during migration):
//   GATEWAY_PORT (default 5030) — the unified WS listener.
//   GATEWAY_MODE=public         — per-room VN, public bind, no forwards
//                                 (same meaning as the three legacy trees).
// TLS (STM32 path, additive WSS listener, unchanged semantics):
//   TLS_PORT / TLS_CERT / TLS_KEY env, or --tls-port/--tls-cert/--tls-key.

import (
	"flag"
	"os"
)

const defaultPort = "5030"

func gatewayPort() string {
	if p := os.Getenv("GATEWAY_PORT"); p != "" {
		return p
	}
	return defaultPort
}

func isPublicMode() bool {
	return os.Getenv("GATEWAY_MODE") == "public"
}

var (
	tlsPortFlag string
	tlsCertFlag string
	tlsKeyFlag  string
)

func parseFlags() {
	flag.StringVar(&tlsPortFlag, "tls-port", "", "also serve HTTPS/WSS on this port (e.g. 5031)")
	flag.StringVar(&tlsCertFlag, "tls-cert", "", "TLS certificate file (PEM) for the WSS listener")
	flag.StringVar(&tlsKeyFlag, "tls-key", "", "TLS private key file (PEM) for the WSS listener")
	flag.Parse()
}

func tlsConfig() (port, cert, key string) {
	port, cert, key = os.Getenv("TLS_PORT"), os.Getenv("TLS_CERT"), os.Getenv("TLS_KEY")
	if tlsPortFlag != "" {
		port = tlsPortFlag
	}
	if tlsCertFlag != "" {
		cert = tlsCertFlag
	}
	if tlsKeyFlag != "" {
		key = tlsKeyFlag
	}
	return port, cert, key
}
