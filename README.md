# dup-cli is a command line tool to upload files to a server that supports multiple file uploads parallelly.

# How to use:
    1. Add dup-cli to your system:
        - Open your terminal.
        - Type `cargo install dup-cli` and press Enter.
        - Wait for the installation to complete.
    2. Add environment variables to your system if you want to use them as default:
        - `UPLOAD_URL` - The exact endpoint URL to upload files to (not just the host).
        - To set an environment variable, open your terminal and type:
            - For Linux/macOS: `export UPLOAD_URL=http://your-upload-url.com/endpoint`
            - For Windows: `setx UPLOAD_URL http://your-upload-url.com/endpoint`
    3. If you want to use a specific host for a specific folder, you can use the `-H` option:
        - Example: `dup-cli -H http://hostname.com/endpoint folder_name file1 file2`
    
❗**NOTES**: Make sure to add the path to `.cargo/bin` in your PATH variable:
    - For Linux/macOS: Add `export PATH="$HOME/.cargo/bin:$PATH"` to your `.bashrc` or `.zshrc` file.
    - For Windows: Add the path to `.cargo/bin` in your system environment variables.

# Example:
    #### Upload a single file or multiple files:
    `dup-cli README.md LICENSE.md`
    #### Upload files to a specific folder:
    `dup-cli -H http://hostname.com folder_name README.md LICENSE.md`

# Installation:
```cargo install dup-cli```

# Usages:

Command:    
`dup-cli [OPTIONS] [FILES||FOLDERS]`

    #### OPTIONS:
    -H, --host: specify the host for a specific folder. Example: `-H http://abc.xyz/api/v1/upload`
    -h, --help: print help information
    -V, --version: print version information
    -p, --progress: show progress bar
    -c, --category: specify the category for a specific kind of upload: multipart, or binary [default: multipart] [values: multipart, binary]

# Future Features

We are planning to add the following features in future releases:

- **Support for additional file transfer protocols**: Including FTP, SFTP, and SCP.
- **Enhanced security features**: Such as encryption of files during transfer.
- **Improved user interface**: A more user-friendly command-line interface with better error messages and help documentation.
- **Integration with cloud storage services**: Direct uploads to services like AWS S3, Google Cloud Storage, and Azure Blob Storage.
- **Automated retry mechanism**: Automatically retry failed uploads.
- **Scheduling uploads**: Schedule uploads to occur at specific times.
- **Detailed logging and reporting**: More detailed logs and reports on upload activities.
- **Support for downloading and syncing**: Ability to download files from the server and sync local files with the server.



