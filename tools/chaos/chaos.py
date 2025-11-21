#!/usr/bin/env python3
import argparse
import random
import subprocess
import sys
import time
from typing import List


DEFAULT_TARGETS = ["auth-api", "directory-api", "admin-api", "node-agent"]
INFRA_SERVICES = {"prometheus", "grafana", "jaeger", "postgres", "redis", "alertmanager"}


def run(cmd: List[str]) -> subprocess.CompletedProcess:
	"""Run a command and raise on non-zero exit."""
	result = subprocess.run(cmd, capture_output=True, text=True)
	if result.returncode != 0:
		raise RuntimeError(f"Command failed: {' '.join(cmd)}\nstdout:\n{result.stdout}\nstderr:\n{result.stderr}")
	return result


def choose_target(services: List[str]) -> str:
	return random.choice(services)


def stop_start(service: str, downtime: int) -> None:
	print(f"[chaos] Stopping {service}")
	run(["docker", "compose", "stop", service])
	time.sleep(downtime)
	print(f"[chaos] Starting {service}")
	run(["docker", "compose", "start", service])


def pause_unpause(service: str, downtime: int) -> None:
	print(f"[chaos] Pausing {service}")
	run(["docker", "compose", "pause", service])
	time.sleep(downtime)
	print(f"[chaos] Unpausing {service}")
	run(["docker", "compose", "unpause", service])


def main() -> None:
	parser = argparse.ArgumentParser(description="Simple chaos tool for docker compose stack")
	parser.add_argument("--iterations", type=int, default=5, help="Number of failure iterations")
	parser.add_argument("--downtime", type=int, default=30, help="Seconds to keep service down/paused")
	parser.add_argument("--interval", type=int, default=60, help="Seconds to wait between iterations")
	parser.add_argument(
		"--mode", choices=["stop", "pause"], default="stop", help="Failure mode: stop/start or pause/unpause"
	)
	parser.add_argument(
		"--services",
		nargs="*",
		default=DEFAULT_TARGETS,
		help=f"Services to target (default: {', '.join(DEFAULT_TARGETS)})",
	)
	parser.add_argument(
		"--include-infra",
		action="store_true",
		help=f"Allow targeting infra services ({', '.join(sorted(INFRA_SERVICES))})",
	)
	args = parser.parse_args()

	targets = args.services
	if not args.include_infra:
		targets = [s for s in targets if s not in INFRA_SERVICES]
	if not targets:
		print("No eligible services to target after filtering; exiting.", file=sys.stderr)
		sys.exit(2)

	action = stop_start if args.mode == "stop" else pause_unpause

	print(f"[chaos] Starting chaos: iterations={args.iterations}, downtime={args.downtime}s, interval={args.interval}s")
	print(f"[chaos] Target pool: {targets}")
	for i in range(1, args.iterations + 1):
		target = choose_target(targets)
		print(f"[chaos] Iteration {i}/{args.iterations}: targeting {target}")
		try:
			action(target, args.downtime)
		except Exception as e:
			print(f"[chaos] Error during action on {target}: {e}", file=sys.stderr)
		if i < args.iterations:
			time.sleep(args.interval)
	print("[chaos] Done.")


if __name__ == "__main__":
	main()


