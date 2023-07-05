from concurrent.futures import ThreadPoolExecutor
from dataclasses import dataclass
from pathlib import Path
from typing import AsyncGenerator, Optional, Tuple
import pytest_asyncio
import asyncio
import subprocess
from asyncio.subprocess import Process
from subprocess import check_output
import os
import pytest
import signal
import socket
import random
import logging

logging.basicConfig(
    level=logging.DEBUG,
    format="%(asctime)s [%(levelname)s] %(message)s",
    handlers=[logging.StreamHandler()],
)

base_path = Path(__file__).parent.parent

SSH_OPTS = "-o ConnectTimeout=60 -o ConnectionAttempts=4 -o UserKnownHostsFile=/dev/null -o StrictHostKeyChecking=no"


async def wait_for_port(host: str, port: int, delay: int = 5) -> None:
    while True:
        sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
        sock.setblocking(False)
        try:
            await asyncio.get_event_loop().sock_connect(sock, (host, port))
            sock.shutdown(socket.SHUT_RDWR)
            break
        except (socket.error, OSError, ConnectionRefusedError) as err:
            await asyncio.sleep(delay)
        finally:
            sock.close()


def generate_mac() -> str:
    return "52:54:00:%02x:%02x:%02x" % (
        random.randint(0, 255),
        random.randint(0, 255),
        random.randint(0, 255),
    )


@dataclass
class Host:
    process: Process
    hostname: str
    address: str
    gateway: str
    pidfile: str

    async def kill(self) -> None:
        # NOTE: Lots of issues with killing qemu instances, openwrt runs as an expect script
        #       sudoing qemu, ubuntu uses a bash script, killing the process of the scripts does
        #       not kill the qemu instances either, we need to keep track of the qemu process (via -pidfile
        #       of qemu) and then kill it, everything else I tried is unreliable like using a trap
        #       in the scripts, process.kill/terminate especially is completely unreliable
        #       (all qemu instances run with snapshot mode, so there is never any data to loose)
        if self.process:
            logging.info(f"kill openwrt qemu instance pid={self.process.pid}")
            process = await asyncio.create_subprocess_shell(
                f"sudo pkill -P {self.process.pid}",
                stdout=subprocess.DEVNULL,
                stderr=subprocess.DEVNULL,
                stdin=subprocess.DEVNULL,
            )
            await process.communicate()
            process = await asyncio.create_subprocess_shell(
                f"sudo kill {self.process.pid}",
                stdout=subprocess.DEVNULL,
                stderr=subprocess.DEVNULL,
                stdin=subprocess.DEVNULL,
            )
            await process.communicate()
        logging.info(f"kill openwrt qemu instance pidfile={self.pidfile}")
        process = await asyncio.create_subprocess_shell(
            f"sudo kill $(sudo cat {self.pidfile})",
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
            stdin=subprocess.DEVNULL,
        )
        await process.communicate()
        await asyncio.sleep(3)

    async def run(self, command: str) -> Tuple[int, str, str]:
        key_filename = str(base_path / "ssh_key")
        process = await asyncio.create_subprocess_shell(
            f"ssh {SSH_OPTS} -i {key_filename} root@{self.address} {command}",
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            stdin=subprocess.DEVNULL,
        )
        stdout, stderr = await process.communicate()
        stdout, stderr = stdout.decode("utf-8"), stderr.decode("utf-8")
        logging.info(f"host: {self.address} ({self.hostname})")
        logging.info(f"command: {command}")
        logging.info(f"status: {process.returncode}")
        logging.info(f"stdout: {stdout.strip()}")
        logging.info(f"stderr: {stderr.strip()}")
        status = -1 if process.returncode is None else process.returncode
        return status, stdout, stderr

    async def upload(self, source: Path, target: str) -> int:
        logging.info(f"upload {str(source)} to {target} on {self.address}")
        key_filename = str(base_path / "ssh_key")
        process = await asyncio.create_subprocess_shell(
            f"scp {SSH_OPTS} -i {key_filename} {str(source)} root@{self.address}:{target}",
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            stdin=subprocess.DEVNULL,
        )
        stdout, stderr = await process.communicate()
        stdout, stderr = stdout.decode("utf-8"), stderr.decode("utf-8")
        logging.info(f"host: {self.address} ({self.hostname})")
        logging.info(f"status: {process.returncode}")
        logging.info(f"stdout: {stdout.strip()}")
        logging.info(f"stderr: {stderr.strip()}")
        status = -1 if process.returncode is None else process.returncode
        return status

    async def ping(self, target_host: str) -> bool:
        status, _, _ = await self.run(f"ping -W 2 -c 1 {target_host}")
        return status == 0
