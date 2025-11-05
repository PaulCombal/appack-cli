Welcome to AppPack
===

You are about to package your first application for AppPack.
This Readme file and all the files in this folder will be part of your AppPack, so it's up to you to edit them to your
liking.

### Getting started

Step one is to boot up your operating system and install it on disk.
As you see `AppPack.yaml` contains the command arguments which will be passed to Qemu to start the VM. The
`install_append` section will only be appended to the Qemu command when launching the VM for the first time.
The main difference with other boot options is the definition of the installation medium. Make sure to place your
installation files (most likely `.iso` files) according to this section.

To launch the VM for the first time and proceed with the OS installation and configuration:

* `appack creator boot-install`

It is essential to set up RDP access during that time. You will not be able to access the VM otherwise.

After installing the OS, shut it down completely. You should then be able to boot it back up using the following
command.

* `appack creator boot`

You might have noticed the `configure_append` section. It is used in this boot mode to allow the AppPack creator to
interact with your VM.
For now, it is recommended not to edit it and not to interfere with the `qmp-appack.sock` file.

From there on, install your application inside the VM. Make sure not to open the app once installed, as you probably
want it to be left in a pristine state for other users to enjoy.

To get a preview of what your app will look like, you can use AppPack's integrated FreeRDP client.

```
$ snap run --shell appack
$ xfreerdp3 <the command parameters under freerdp_command>
```

The OS chooses the RDP port used; it must not be fixed. It will be logged when starting your "configure" session.

The next step is to take a complete snapshot of the VM's current state. The VM will be immediately shut down afterward.
AppPack will create an internal snapshot of the VM's current state, so if you want to go back and edit your app you would have to undo the snapshot first.

AppPack uses snapshots a lot, so make sure the OS is not downloading any updates or running any background tasks when
you decide to take this final snapshot. If this happens, users will be re-running the background tasks or updates every
time they start the application.

Additionally, be aware that once the snapshot is created, users will not be able to adjust the VM's configuration
(e.g. add a new disk, change the CPU count, etc.).

Run the following command while your VM is still running and ready to open your desired program.

* `appack creator snapshot`

If the snapshot was created successfully, a package will be created following the id and version specified in
`AppPack.yaml`. This package is ready to be distributed to users.

If you want to re-package your app without using the creator, you can create a zip archive including the same files
as the package would have with the creator.

### Limitations

AppPack is still in development, and there are a few things that are not yet supported. If you encounter any issues,
or limitations that prevent offering the best user experience, please open an issue on
the [GitHub repository](https://github.com/TODO).