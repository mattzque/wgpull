UBUNTU_20_04_IMAGE = qemu/packer/ubuntu_20_04_amd64
OPENWRT_IMAGE = qemu/openwrt/image
OPENWRT_VERSION = "22.03.5"
OPENWRT_URL = "https://archive.openwrt.org/releases/${OPENWRT_VERSION}/targets/armvirt/32"

ADDRESS = 10.180.0.2
GATEWAY = 10.180.0.1
HOSTNAME = openwrt

.PHONY: all prepare test

all: test

prepare: pyenv $(UBUNTU_20_04_IMAGE) $(OPENWRT_IMAGE)

$(UBUNTU_20_04_IMAGE):
	@echo "Building Ubuntu 20.04 image (this takes 7:23 minutes for me, excluding iso download)"
	cd qemu/packer/ && packer build ubuntu_20_04_amd64.pkr.hcl

$(OPENWRT_IMAGE):
	rm -Rf ${OPENWRT_IMAGE}
	mkdir ${OPENWRT_IMAGE}
	wget -O ${OPENWRT_IMAGE}/openwrt-armvirt-32-zImage "${OPENWRT_URL}/openwrt-${OPENWRT_VERSION}-armvirt-32-zImage"
	wget -O ${OPENWRT_IMAGE}/openwrt-armvirt-32-rootfs-ext4.img.gz "${OPENWRT_URL}/openwrt-${OPENWRT_VERSION}-armvirt-32-rootfs-ext4.img.gz"
	gunzip ${OPENWRT_IMAGE}/openwrt-armvirt-32-rootfs-ext4.img.gz
	qemu-img convert -f raw -O qcow2 \
		${OPENWRT_IMAGE}/openwrt-armvirt-32-rootfs-ext4.img \
		${OPENWRT_IMAGE}/openwrt-armvirt-32-rootfs-ext4.qcow2
	cd qemu/openwrt/ && ./setup_openwrt_qemu.sh ${ADDRESS} ${GATEWAY} ${HOSTNAME}

pyenv:
	cd pytests/ && python3 -m venv venv || true
	cd pytests/ && . venv/bin/activate && python3 -m pip install -r requirements.txt

test-ubuntu: prepare
	@echo "Running integration tests..."
	cd pytests/ && . venv/bin/activate && pytest -s -o log_cli=true -o log_cli_level=INFO test_ubuntu_network.py

test-openwrt: prepare
	@echo "Running integration tests..."
	cd pytests/ && . venv/bin/activate && pytest -s -o log_cli=true -o log_cli_level=INFO test_openwrt_network.py

test: test-ubuntu test-openwrt

clean-ubuntu:
	rm -Rf qemu/packer/ubuntu_20_04_amd64

clean-openwrt:
	rm -Rf ${OPENWRT_IMAGE}

clean-pyenv:
	rm -Rf pytests/venv

clean: clean-ubuntu clean-openwrt clean-pyenv
