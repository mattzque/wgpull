from concurrent.futures import ThreadPoolExecutor
from dataclasses import dataclass
from pathlib import Path
from typing import AsyncGenerator, Optional, Tuple
import pytest_asyncio
import asyncio
import subprocess
from asyncio.subprocess import Process
from subprocess import check_output
import pytest
import signal
import socket
import random
import logging

from utils import SSH_OPTS, wait_for_port, generate_mac, Host, base_path 


pytest_plugins = ("pytest_asyncio",)

fixtures_path = Path(__file__).parent / "fixtures" / "ubuntu"


async def spawn_ubuntu_guest(hostname: str, address: str) -> AsyncGenerator[Host, None]:
    logging.info(f"starting ubuntu qemu instance for {address}")
    cwd = base_path / "qemu" / "packer"
    script = cwd / "start_ubuntu_20_04_qemu.sh"

    host: Optional[Host] = None
    pidfile: str = str(base_path / f"{hostname}.pid")

    try:
        process = await asyncio.create_subprocess_shell(
            f"{script} {address} {pidfile}",
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
            stdin=subprocess.DEVNULL,
            cwd=str(cwd),
        )
        # wait for host to appear (check ssh port)
        await wait_for_port(address, 22)

        host = Host(
            hostname=hostname,
            address=address,
            gateway="",
            process=process,
            pidfile=pidfile,
        )

        status, uname, _ = await host.run("uname -a")
        assert status == 0, "failed to run ubuntu host"
        logging.info(f"ubuntu host started ({repr(uname)})")

        yield host

        await host.kill()

    except Exception as e:
        if host:
            await host.kill()
        raise e


@pytest_asyncio.fixture
async def ubuntu_host_lighthouse() -> AsyncGenerator[Host, None]:
    async for host in spawn_ubuntu_guest("lighthouse", "10.180.0.2"):
        yield host


@pytest_asyncio.fixture
async def ubuntu_host_node1() -> AsyncGenerator[Host, None]:
    async for host in spawn_ubuntu_guest("node1", "10.180.0.3"):
        yield host


@pytest_asyncio.fixture
async def ubuntu_host_node2() -> AsyncGenerator[Host, None]:
    async for host in spawn_ubuntu_guest("node2", "10.180.0.4"):
        yield host


@dataclass
class Network:
    lighthouse: Host
    node1: Host
    node2: Host

    def __iter__(self):
        yield self.lighthouse
        yield self.node1
        yield self.node2


@pytest_asyncio.fixture
async def ubuntu_network(
    ubuntu_host_lighthouse: Host,
    ubuntu_host_node1: Host,
    ubuntu_host_node2: Host,
) -> AsyncGenerator[Network, None]:
    yield Network(
        lighthouse=ubuntu_host_lighthouse,
        node1=ubuntu_host_node1,
        node2=ubuntu_host_node2,
    )


@pytest.mark.asyncio
async def test_ubuntu_network(
    ubuntu_network: Network,
) -> None:
    """
    Setup a lighthouse and two nodes, the lighthouse itself is setup as a node itself.

    This only tests Ubuntu hosts, using the built .deb package.

    Steps:
    - start hosts
    - assert connectivity between all hosts
    - upload deb package and install wgpull
    - upload wgpull.conf files
    - start wgpull lighthouse and node services on each host
    - assert availability of lighthouse service
    - assert network connectivity between all hosts
    - change configured address on node1
    - restart node service on node1
    - assert availability of node1 service new address
    """
    print("running... waiting 30 seconds")
    await asyncio.sleep(30)

    assert await ubuntu_network.lighthouse.ping(
        "10.180.0.2"
    ), f"failed to ping lighthouse from lighthouse"
    assert await ubuntu_network.lighthouse.ping(
        "10.180.0.3"
    ), f"failed to ping node1 from lighthouse"
    assert await ubuntu_network.lighthouse.ping(
        "10.180.0.4"
    ), f"failed to ping node2 from lighthouse"

    assert await ubuntu_network.node1.ping(
        "10.180.0.2"
    ), f"failed to ping lighthouse from node1"
    assert await ubuntu_network.node1.ping(
        "10.180.0.3"
    ), f"failed to ping node1 from node1"
    assert await ubuntu_network.node1.ping(
        "10.180.0.4"
    ), f"failed to ping node2 from node1"

    assert await ubuntu_network.node2.ping(
        "10.180.0.2"
    ), f"failed to ping lighthouse from node2"
    assert await ubuntu_network.node2.ping(
        "10.180.0.3"
    ), f"failed to ping node1 from node2"
    assert await ubuntu_network.node2.ping(
        "10.180.0.4"
    ), f"failed to ping node2 from node2"

    for host in ubuntu_network:
        logging.info(f"uploading wgpull package to host {host.hostname}")
        status = await host.upload(
            base_path.parent.parent / "package" / "wgpull_0.1.0_amd64.deb",
            "/root/wgpull.deb",
        )
        assert status == 0, "failed to upload wgpull package"
        status, _, _ = await host.run("ls -lah /root")
        assert status == 0, "failed to list /root"
        status, _, _ = await host.run("apt-get install -y wireguard")
        status, _, _ = await host.run("dpkg -i /root/wgpull.deb")
        assert status == 0, "failed to install wgpull package"

    logging.info(f"uploading lighthouse configuration fixture")
    status = await ubuntu_network.lighthouse.upload(
        fixtures_path / "lighthouse.conf", "/etc/wgpull/wgpull.conf"
    )
    assert status == 0, "failed to upload lighthouse configuration fixture"

    logging.info(f"uploading node1 configuration fixture")
    status = await ubuntu_network.node1.upload(
        fixtures_path / "node1.conf", "/etc/wgpull/wgpull.conf"
    )
    assert status == 0, "failed to upload node1 configuration fixture"

    logging.info(f"uploading node2 configuration fixture")
    status = await ubuntu_network.node2.upload(
        fixtures_path / "node2.conf", "/etc/wgpull/wgpull.conf"
    )
    assert status == 0, "failed to upload node2 configuration fixture"

    for host in ubuntu_network:
        logging.info(f"start wgpull-node service on {host.hostname}")
        status, _, _ = await host.run("systemctl start wgpull-node")
        assert status == 0, "failed to start wgpull-node service"

    logging.info(f"start wgpull-lighthouse service")
    status, _, _ = await ubuntu_network.lighthouse.run(
        "systemctl start wgpull-lighthouse"
    )
    assert status == 0, "failed to start wgpull-lighthouse service"

    await wait_for_port(ubuntu_network.lighthouse.address, 2001)

    print("giving time for network establishment... waiting 30 seconds")
    await asyncio.sleep(30)

    _, stdout, _ = await ubuntu_network.lighthouse.run("journalctl -u wgpull-lighthouse -n 100")
    logging.info(f"lighthouse logs: {stdout}")
    _, stdout, _ = await ubuntu_network.lighthouse.run("journalctl -u wgpull-node -n 100")
    logging.info(f"lighthouse logs: {stdout}")
    _, stdout, _ = await ubuntu_network.node1.run("journalctl -u wgpull-node -n 100")
    logging.info(f"lighthouse logs: {stdout}")
    _, stdout, _ = await ubuntu_network.node2.run("journalctl -u wgpull-node -n 100")
    logging.info(f"lighthouse logs: {stdout}")

    assert await ubuntu_network.lighthouse.ping(
        "10.190.0.1"
    ), f"failed to ping lighthouse from lighthouse"
    assert await ubuntu_network.lighthouse.ping(
        "10.190.0.2"
    ), f"failed to ping node1 from lighthouse"
    assert await ubuntu_network.lighthouse.ping(
        "10.190.0.3"
    ), f"failed to ping node2 from lighthouse"

    assert await ubuntu_network.node1.ping(
        "10.190.0.1"
    ), f"failed to ping lighthouse from node1"
    assert await ubuntu_network.node1.ping(
        "10.190.0.2"
    ), f"failed to ping node1 from node1"
    assert await ubuntu_network.node1.ping(
        "10.190.0.3"
    ), f"failed to ping node2 from node1"

    assert await ubuntu_network.node2.ping(
        "10.190.0.1"
    ), f"failed to ping lighthouse from node2"
    assert await ubuntu_network.node2.ping(
        "10.190.0.2"
    ), f"failed to ping node1 from node2"
    assert await ubuntu_network.node2.ping(
        "10.190.0.3"
    ), f"failed to ping node2 from node2"
