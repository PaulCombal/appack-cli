TODO:
/vmconnect ????


Here’s how to **dump a live VM’s memory and device state** to a `vmstate.bin` file directly from QEMU.

---

## 🧠 Goal

You want to capture the *entire runtime state* (CPU, RAM, devices, etc.) of a running VM into a binary snapshot — the same data format QEMU would use for live migration or `-incoming`.

---

## ✅ Option 1: Using the QEMU monitor (`migrate` command)

You can do this interactively or via QMP (QEMU Machine Protocol).

### Step-by-step (human-readable monitor)

1. **Connect to the QEMU monitor**

  * If you started QEMU with a monitor enabled (e.g. `-monitor stdio` or `-monitor unix:/tmp/qmp.sock,server,nowait`), connect to it:

    ```bash
    nc -U /tmp/qmp.sock
    ```

    or use `virsh qemu-monitor-command` if under libvirt:

    ```bash
    virsh qemu-monitor-command <vmname> --hmp
    ```

2. **Pause the VM (optional but safer)**

   ```bash
   (qemu) stop
   ```

3. **Dump the migration state to file**

   ```bash
   (qemu) migrate "exec:cat > /path/to/vmstate.bin"
   ```

   or

   ```bash
   (qemu) migrate "file:/path/to/vmstate.bin"
   ```

   QEMU will now write the VM’s live state to that file.

4. **Wait for it to finish**
   You can check progress:

   ```bash
   (qemu) info migrate
   ```

   Once the migration completes, you’ll see:

   ```
   completed
   ```

5. **Resume (optional)**

   ```bash
   (qemu) cont
   ```

That’s it — now you have a `vmstate.bin` you can later restore with:

```bash
qemu-system-x86_64 [your normal options] -incoming file:/path/to/vmstate.bin
```

---

## ✅ Option 2: Using QMP (for scripting or automation)

If you’re automating, you can use the QMP JSON protocol.

Example (using `socat` or Python):

```json
{ "execute": "migrate", "arguments": { "uri": "file:/path/to/vmstate.bin" } }
```

Then poll:

```json
{ "execute": "query-migrate" }
```

until `"status": "completed"`.

---

## ⚠️ Notes & Gotchas

* You must **use identical QEMU versions and device configs** when restoring (`-incoming`), or it will fail.
* The **disk images** are *not* included — they must be in the same state they were during the dump.
* The dump is **not portable** between architectures or major QEMU releases.
* You can use `-S` (do not start CPU) when restoring:

  ```bash
  qemu-system-x86_64 -S -incoming file:/path/to/vmstate.bin
  ```

  Then `(qemu) cont` when ready.

---

## MAKING A VM

The most common and effective way to connect an external visualizer to a headless QEMU instance is by enabling a **VNC** or **SPICE** server within QEMU.

You **do need to specify an address and port** (or a display number) for the visualizer to connect to.

### 1\. Connecting via VNC or SPICE

QEMU has built-in support for VNC (Virtual Network Computing) and SPICE (Simple Protocol for Independent Computing Environments), which you can use for graphical access.

#### QEMU Command Line

When starting your headless QEMU VM, you need to add the appropriate option:

* **VNC over a TCP port:** Use the `-vnc` option. The number after the colon is the VNC *display* number. The VNC port will be **5900 + display number**.
    * To listen on all interfaces (for remote connection) on port 5901:
      ```bash
      qemu-system-x86_64 ... -vnc 0.0.0.0:1
      ```
      (Here, `:1` means port $5900 + 1 = 5901$).
    * To listen only on the local machine (for connecting from the host) on port 5900:
      ```bash
      qemu-system-x86_64 ... -vnc :0
      ```
      (Here, `:0` means port $5900 + 0 = 5900$).
* **SPICE:** Use the `-spice` option. This is generally preferred over VNC as it offers better features like copy/paste, audio, and better performance.
    * To listen on all interfaces on a specific port (e.g., 5900):
      ```bash
      qemu-system-x86_64 ... -spice port=5900,addr=0.0.0.0,disable-ticketing
      ```
      (You can add `password=...` instead of `disable-ticketing` for security).

You would then use a standard VNC client (like **RealVNC Viewer, TigerVNC Viewer**) or a SPICE client (**`remote-viewer` / `virt-viewer`**) to connect to the IP address or hostname of your QEMU host machine on the specified port.

-----

### 2\. Using `virt-manager`

Yes, you **can** use `virt-manager` to connect to a headless QEMU VM, but it generally requires the VM to be managed by **libvirt**.

* **If your QEMU VM is managed by libvirt:**

    * If `virt-manager` is running on the same host, it will usually detect the VM and allow you to open the console viewer.
    * If `virt-manager` is on a **remote machine**, you can configure it to connect to the headless host via **SSH**. `virt-manager` will then manage the VM and automatically open the graphical console (VNC or SPICE) tunnelled securely over SSH, making the port configuration less critical for manual connection.

* **If you are running QEMU directly without libvirt:**

    * `virt-manager` cannot directly manage a standalone QEMU process.
    * However, the `virt-viewer` utility (often installed with `virt-manager`) **can** be used as a simple external visualizer for VNC or SPICE sessions. You would launch it directly with the connection details:
      ```bash
      virt-viewer --connect vnc://<QEMU_HOST_IP>:<PORT>
      # OR
      virt-viewer --connect spice://<QEMU_HOST_IP>:<PORT>
      ```
