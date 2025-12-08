This is the documentation meant for tinkerers and developers alike.

You are at the right place if you want to package an application with AppPack, contribute to the project or just want to learn more about it.

### Thanks

Before jumping into the documentation, I want to thank the maintainers of the following projects for their work and ispiration:
* Qemu
* KVM
* WinApps
* FreeRDP
* Canonical

### How does it work?

AppPack didn't invent the wheel. It's only a wrapper around Qemu + KVM + RDP.

AppPack leverages Qemu + KVM to run the application inside a virtual machine. Yes, an entire OS is running inside the VM.
This might seem overkill to some, but keep in mind that with KVM, it is possible to reach near native performance.
It is also possible to enable RAM ballooning to lower the cost of running the VM.

When the VM is properly configured, you host connects to the VM via RDP and the application is running inside it.

### What are the limitations?

Unfortunately, I did not manage to enable VirGL for AppPacks.
AppPack relies a lot on taking snapshots of the VM to be able to restore the state of the application.
Unfortunately, some VM devices (like VirGL) do not support snapshots. ("virgl is not yet migratable").

I also could not think of a reasonable way to package AppPacks with a GPU passthrough configuration, although I imagine very few people would want to do that.

Also, when connected via RDP to the VM, the animations seem to be a bit janky.
I suspect this is du to the way RDP doesn't give us the actual GPU driver for an RDP session.
I tried changing some settings in the VM, and even enabling RemoteFX, but to no avail.

### How do you package an application?

Start by creating a new folder and moving into it.

Then launch `appack new` to scaffold the required files.

Inside the AppPack folder, you will find a `Readme.md` file with generic instructions on how to package your application. 

You might have noticed the some bash-like variables are present in `AppPackBuildConfig.yaml`.

Here are the replacement values:
* `$HOME`: Your home directory
* `$RDP_PORT`: The port you want to use for RDP connections
* `$APPACK_LAUNCH_CMD`: You should use it for the Exec line in your .desktop files
* `$IMAGE_FILE_PATH`: The path to the image file you want to use
* `$ICON_DIR`: The path to the icon directory for your application
* `$ICON_FULL_PATH`: The full path to the icon you want to use for your desktop entry
* `$WHITESPACE`: A whitespace character (can be used for escaping a space character)

These are only replaced when applicable.

In addition, the is one "sort of" function you can use for desktop entries definition:

* `$TO_WIN_ESCAPED_PATH**str**`: Converts a Unix path to a Windows-compatible path, prefixed with `\\tsclient\home`

The available snapshot modes in `AppPackBuildConfig.yaml` are:
* `NeverLoad`: Never take a snapshot, always cold boot the VM
* `Never`: Always load the initial state snapshot, but never take a new one
* `OnClose`: Always load the last snapshot, take a new one when the VM is closed

You can find a packaging example for a famous office suite [here](https://github.com/PaulCombal/appack-365).

### How do I contribute?

Please contact me or open an issue before submitting a pull request.
Let me know what your issue is, and how you want to fix it first.

### How is the project structured?

As you can see the project is quite simple, yet the code quality is not quite high.

The three modules are the following:
* `internal`: The core logic of each AppPack command
* `types`: The type definitions for the AppPack configuration files and more
* `utils`: Utility functions

The script `rebuild_snap.sh` is used to rebuild the snap package locally. It is simply a wrapper around `snapcraft`.

For now there is not much more to it, feel free to open an issue if you have any questions.