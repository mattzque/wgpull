# Ubuntu 20.04.6 LTS

source "qemu" "ubuntu_20_04_amd64" {
  vm_name           = "ubuntu_20_04_amd64.qcow2"
  iso_url           = "http://www.releases.ubuntu.com/20.04/ubuntu-20.04.6-live-server-amd64.iso"
  iso_checksum      = "sha256:b8f31413336b9393ad5d8ef0282717b2ab19f007df2e9ed5196c13d8f9153c8b"
  memory            = 1024
  disk_image        = false
  output_directory  = "ubuntu_20_04_amd64"
  accelerator       = "kvm"
  disk_size         = "20G"
  disk_interface    = "virtio"
  format            = "qcow2"
  net_device        = "virtio-net"
  boot_wait         = "3s"
  boot_command      = [
    # Make the language selector appear...
    " <up><wait>",
    # ...then get rid of it
    " <up><wait><esc><wait>",

    # Go to the other installation options menu and leave it
    "<f6><wait><esc><wait>",

    # Remove the kernel command-line that already exists
    "<bs><bs><bs><bs><bs><bs><bs><bs><bs><bs><bs><bs><bs><bs><bs><bs><bs><bs><bs><bs>",
    "<bs><bs><bs><bs><bs><bs><bs><bs><bs><bs><bs><bs><bs><bs><bs><bs><bs><bs><bs><bs>",
    "<bs><bs><bs><bs><bs><bs><bs><bs><bs><bs><bs><bs><bs><bs><bs><bs><bs><bs><bs><bs>",

    # Add kernel command-line and start install
    "/casper/vmlinuz ",
    "initrd=/casper/initrd ",
    "autoinstall ",
    "ds=nocloud-net;s=http://{{.HTTPIP}}:{{.HTTPPort}}/ ",
    "<enter>"
  ]
  http_directory    = "http"
  shutdown_command  = "echo 'packer' | sudo -S shutdown -P now"
  ssh_username      = "admin"
  ssh_password      = "Chae9cahb5"
  ssh_timeout       = "20m"
  headless          = true
}

build {
  name = "ubuntu_20_04_amd64"
  sources = ["source.qemu.ubuntu_20_04_amd64"]

  provisioner "shell" {
    script = "after_install_setup.sh"
    execute_command = "sudo bash {{ .Path }}"
  }
}
