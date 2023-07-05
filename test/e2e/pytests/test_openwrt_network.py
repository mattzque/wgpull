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

fixtures_path = Path(__file__).parent / "fixtures" / "openwrt"


async def spawn_openwrt_guest(
    hostname: str, address: str, internal_address: str
) -> AsyncGenerator[Host, None]:
    logging.info(f"starting openwrt qemu instance for {address}")
    cwd = base_path / "qemu" / "openwrt"
    script = cwd / "start_openwrt_qemu.sh"

    gateway = "10.180.0.1"
    host: Optional[Host] = None
    mac1: str = generate_mac()
    mac2: str = generate_mac()
    pidfile: str = str(base_path / f"{hostname}.pid")

    try:
        process = await asyncio.create_subprocess_shell(
            f"{script} {internal_address} {address} {gateway} {hostname} {mac1} {mac2} {pidfile}",
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
            gateway=gateway,
            process=process,
            pidfile=pidfile,
        )

        status, uname, _ = await host.run("uname -a")
        assert status == 0, "failed to run openwrt host"
        logging.info(f"openwrt host started ({repr(uname)})")

        yield host

        await host.kill()

    except Exception as e:
        if host:
            await host.kill()
        raise e


@pytest_asyncio.fixture
async def openwrt_host_lighthouse() -> AsyncGenerator[Host, None]:
    async for host in spawn_openwrt_guest("lighthouse", "10.180.0.2", "10.40.1.1"):
        yield host


@pytest_asyncio.fixture
async def openwrt_host_node1() -> AsyncGenerator[Host, None]:
    async for host in spawn_openwrt_guest("node1", "10.180.0.3", "10.40.2.1"):
        yield host


@pytest_asyncio.fixture
async def openwrt_host_node2() -> AsyncGenerator[Host, None]:
    async for host in spawn_openwrt_guest("node2", "10.180.0.4", "10.40.3.1"):
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
async def openwrt_network(
    openwrt_host_lighthouse: Host,
    openwrt_host_node1: Host,
    openwrt_host_node2: Host,
) -> AsyncGenerator[Network, None]:
    yield Network(
        lighthouse=openwrt_host_lighthouse,
        node1=openwrt_host_node1,
        node2=openwrt_host_node2,
    )


@pytest.mark.asyncio
async def test_openwrt_network(
    openwrt_network: Network,
) -> None:
    """
    Setup a lighthouse and two nodes, the lighthouse itself is setup as a node itself.

    This only tests OpenWRT hosts, using the built .ipk package.

    Steps:
    - start hosts
    - assert connectivity between all hosts
    - upload ipk package and install wgpull
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

    assert await openwrt_network.lighthouse.ping(
        "10.180.0.2"
    ), f"failed to ping lighthouse from lighthouse"
    assert await openwrt_network.lighthouse.ping(
        "10.180.0.3"
    ), f"failed to ping node1 from lighthouse"
    assert await openwrt_network.lighthouse.ping(
        "10.180.0.4"
    ), f"failed to ping node2 from lighthouse"

    assert await openwrt_network.node1.ping(
        "10.180.0.2"
    ), f"failed to ping lighthouse from node1"
    assert await openwrt_network.node1.ping(
        "10.180.0.3"
    ), f"failed to ping node1 from node1"
    assert await openwrt_network.node1.ping(
        "10.180.0.4"
    ), f"failed to ping node2 from node1"

    assert await openwrt_network.node2.ping(
        "10.180.0.2"
    ), f"failed to ping lighthouse from node2"
    assert await openwrt_network.node2.ping(
        "10.180.0.3"
    ), f"failed to ping node1 from node2"
    assert await openwrt_network.node2.ping(
        "10.180.0.4"
    ), f"failed to ping node2 from node2"

    for host in openwrt_network:
        logging.info(f"uploading wgpull package to host {host.hostname}")
        status = await host.upload(
            base_path.parent.parent / "package" / "wgpull-0.1.0.ipk", "/root/wgpull.ipk"
        )
        assert status == 0, "failed to upload wgpull package"
        status, _, _ = await host.run("ls -lah /root")
        assert status == 0, "failed to list /root"
        status, _, _ = await host.run("opkg install /root/wgpull.ipk")
        assert status == 0, "failed to install wgpull package"

    logging.info(f"uploading lighthouse configuration fixture")
    status = await openwrt_network.lighthouse.upload(
        fixtures_path / "lighthouse.conf", "/etc/wgpull/wgpull.conf"
    )
    assert status == 0, "failed to upload lighthouse configuration fixture"

    logging.info(f"uploading node1 configuration fixture")
    status = await openwrt_network.node1.upload(
        fixtures_path / "node1.conf", "/etc/wgpull/wgpull.conf"
    )
    assert status == 0, "failed to upload node1 configuration fixture"

    logging.info(f"uploading node2 configuration fixture")
    status = await openwrt_network.node2.upload(
        fixtures_path / "node2.conf", "/etc/wgpull/wgpull.conf"
    )
    assert status == 0, "failed to upload node2 configuration fixture"

    for host in openwrt_network:
        logging.info(f"start wgpull-node service on {host.hostname}")
        status, _, _ = await host.run("/etc/init.d/wgpull-node start")
        assert status == 0, "failed to start wgpull-node service"

    logging.info(f"start wgpull-lighthouse service")
    status, _, _ = await openwrt_network.lighthouse.run(
        "/etc/init.d/wgpull-lighthouse start"
    )
    assert status == 0, "failed to start wgpull-lighthouse service"

    await wait_for_port(openwrt_network.lighthouse.address, 2001)

    print("giving time for network establishment... waiting 30 seconds")
    await asyncio.sleep(30)

    assert await openwrt_network.lighthouse.ping(
        "10.190.0.1"
    ), f"failed to ping lighthouse from lighthouse"
    assert await openwrt_network.lighthouse.ping(
        "10.190.0.2"
    ), f"failed to ping node1 from lighthouse"
    assert await openwrt_network.lighthouse.ping(
        "10.190.0.3"
    ), f"failed to ping node2 from lighthouse"

    assert await openwrt_network.node1.ping(
        "10.190.0.1"
    ), f"failed to ping lighthouse from node1"
    assert await openwrt_network.node1.ping(
        "10.190.0.2"
    ), f"failed to ping node1 from node1"
    assert await openwrt_network.node1.ping(
        "10.190.0.3"
    ), f"failed to ping node2 from node1"

    assert await openwrt_network.node2.ping(
        "10.190.0.1"
    ), f"failed to ping lighthouse from node2"
    assert await openwrt_network.node2.ping(
        "10.190.0.2"
    ), f"failed to ping node1 from node2"
    assert await openwrt_network.node2.ping(
        "10.190.0.3"
    ), f"failed to ping node2 from node2"
