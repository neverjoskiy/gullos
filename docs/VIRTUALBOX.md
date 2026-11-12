# Running GullOS in Oracle VirtualBox (Windows 11 Host Guide)

GullOS produces two bootable artifacts in every GitHub Actions CI run:
1. **`gullos-x86_64.vdi.zst`** — **Recommended for VirtualBox**. A pre-partitioned, writable `ext4` VirtualBox Disk Image where `sudo pacman -S <package>` and `/home/gull` files persist natively across reboots.
2. **`gullos-x86_64.iso`** — A hybrid BIOS/UEFI Live & Installer image (useful for live testing or bare-metal USB flashing via Rufus/Ventoy).

---

## Step 1: Download & Decompress the Artifact on Windows 11

1. Navigate to `https://github.com/neverjoskiy/gullos/actions` (or the **Releases** page) and download **`gullos-images`** / **`gullos-x86_64.vdi.zst`**.
2. Decompress `gullos-x86_64.vdi.zst` on Windows 11 using any of:
   - **7-Zip** (v23.01+ supports `.zst` natively: Right-click -> *7-Zip* -> *Extract Here*), **NanaZip**, or **WinRAR**.
   - **PowerShell / Winget**:
     ```powershell
     winget install -e --id Facebook.zstd
     zstd -d gullos-x86_64.vdi.zst
     ```
   You now have `gullos-x86_64.vdi` (expands dynamically up to `12 GB` as you install packages).

---

## Step 2: Recommended VirtualBox 7.x VM Configuration

Open **Oracle VirtualBox Manager** on Windows 11 and click **New**:

### 1. Name and Operating System
- **Name**: `GullOS`
- **ISO Image**: *(Leave blank if using `.vdi`, or select `gullos-x86_64.iso` if booting Live ISO)*
- **Type**: `Linux`
- **Subtype / Version**: `Arch Linux (64-bit)` *(or `Other Linux (64-bit)`)*

### 2. System (Motherboard & Processor)
- **Base Memory (RAM)**: `4096 MB` recommended (`2048 MB` minimum; Firefox runs best with `>= 4096 MB`).
- **Boot Order**: Hard Disk first (or Optical first if booting `.iso`).
- **Extended Features**:
  - Check **Enable I/O APIC**
  - Check **Enable EFI (special OSes only)** — GullOS supports **both** UEFI and Legacy BIOS, and UEFI provides crisp native KMS framebuffer resolution immediately at boot!
- **Processors**: `2` to `4` CPUs.
- **Hardware Virtualization**: Ensure **Nested VT-x/AMD-V** / standard virtualization is active. *(Note on Windows 11: if Hyper-V / Memory Integrity is enabled and VirtualBox shows the green turtle icon, GullOS still boots cleanly thanks to software-compatible `wl_shm` Wayland rendering.)*

### 3. Display
- **Video Memory (VRAM)**: Slide all the way right to **`128 MB`**.
- **Graphics Controller**: **`VMSVGA`** *(GullOS includes `CONFIG_DRM_VMWGFX=y`, `CONFIG_DRM_VBOXVIDEO=y`, and `CONFIG_DRM_BOCHS=y` built directly into the kernel, so `VMSVGA` and `VBoxSVGA` both work out-of-the-box)*.
- **Acceleration**: **Enable 3D Acceleration** is **optional** (leave unchecked on Windows 11 if your host GPU driver flickers in VirtualBox — both `labwc` and `gull-term` render smoothly via Wayland shared-memory `wl_shm` without 3D acceleration).

### 4. Storage (Persistent `.vdi` Method — Recommended)
- Under **Controller: SATA** (Type: `AHCI`, Port Count: `2`):
  - Click **Adds Hard Disk** icon -> **Add** -> select your extracted **`gullos-x86_64.vdi`** -> **Choose**.
  - Remove any empty default dynamically created virtual disk if VirtualBox created one.

### 5. Network
- **Adapter 1**: Check **Enable Network Adapter**
- **Attached to**: **`NAT`** (gives instant outbound internet access from your Windows 11 host so `pacman` and `firefox` work immediately).
- **Advanced -> Adapter Type**: **`Intel PRO/1000 MT Desktop (82540EM)`** *(default)* or **`Paravirtualized Network (virtio-net)`** — both drivers (`CONFIG_E1000=y` and `CONFIG_VIRTIO_NET=y`) are compiled directly into the GullOS kernel.

### 6. Audio
- Uncheck **Enable Audio** (GullOS intentionally omits audio daemons for minimal footprint).

---

## Step 3: First Boot Walkthrough

1. Click **Start** in VirtualBox.
2. GRUB boots `GullOS 0.1.0 (x86_64)` into `labwc` and immediately engages **`gull-lock`** (Catppuccin Mocha lockscreen).
3. Type the default password **`gull`** and press **`Enter`**.
4. **`gull-term`** opens automatically running **`gull-sh`**:
   - Notice real-time syntax highlighting: type `pacman` (turns **Green** `#a6e3a1`) vs `notacommand` (turns **Red** `#f38ba8`).
   - Press **`Tab`** to autocomplete commands or file paths.
5. Verify binary `pacman` & reboot persistence:
   ```sh
   sudo pacman -Sy btop
   btop
   ```
   Reboot with `sudo reboot`, unlock `gull-lock`, and run `btop` again — your installed package is still there!
6. Launch Firefox in pure Wayland mode:
   - Press **`Super + B`** (on Windows 11 VirtualBox, `Right Ctrl` is the Host key by default, so Windows key passes through if VirtualBox keyboard capture is active — or simply run `firefox &` inside `gull-term`, or right-click the desktop and click **Firefox Web Browser**).
7. Test raw Linux TTY fallback:
   - In VirtualBox menu: **Input -> Keyboard -> Insert Ctrl-Alt-Del** or switch VT with **`Host (Right Ctrl) + F2`**.
   - Log in as `gull` (password `gull`) and run `gull-term` — it detects no `WAYLAND_DISPLAY` and drops seamlessly into native TTY `gull-sh` without crashing!

---

## Alternative: Installing from `gullos-x86_64.iso` onto a Fresh Virtual Disk or Bare Metal

If you boot `gullos-x86_64.iso` alongside a blank virtual disk (or USB -> NVMe/SATA on real hardware):
1. Unlock `gull-lock` (password `gull`).
2. In `gull-term`, inspect disks with `lsblk`.
3. Install GullOS permanently onto the target drive:
   ```sh
   sudo gull-install /dev/sda
   ```
4. Power off (`sudo poweroff`), eject the ISO from the VirtualBox Optical Drive, and boot from the hard disk.
