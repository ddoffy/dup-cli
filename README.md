# How to use:
    1. Add dup-cli to your system:
        - `cargo install dup-cli`
    2. Add environment variables to your system if you want to use them as
       default:
        - `UPLOAD_URL` - URL to upload files to
    2.a. If you wanna use a specific host for a specific folder, you can use -H
    hostname to specify the host
        - `dup-cli -H hostname folder_name file1 file2`
    
❗**NOTES**: you gotta set path to .cargo/bin in your PATH variable


# Example:
    #### upload file or multiple files 
    `dup-cli README.md LICENSE.md`
    #### upload file or multiple files to a specific folder
    `dup-cli README.md LICENSE.md folder_name`
