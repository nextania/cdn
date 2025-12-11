# Nextania CDN

## Description

This is the repository for the Nextania CDN. It is responsible for the following tasks:
* storing user-generated files/attachments,
* previewing third-party media, and
* serving first-party files.

Notably, it has various supplemental features including:
* virus scanning of uploaded content using ClamAV
* signature verification of file URLs to prevent abuse, and
* image processing.

This server saves files on an S3-compatible object storage service. For more information, refer to the documentation [here](https://nextania.com/developers/services/cdn).

## Contributing

This is a Rust project, so you'll need have the Rust toolchain installed. For more information, refer to the Rust installation guide and documentation [here](https://www.rust-lang.org/).

The contribution guide is located [here](https://nextania.com/developers/contributions); please create one pull request per issue in order to accelerate the review process.

## License

This project is licensed under the [GNU Affero General Public License v3.0](https://github.com/nextania/cdn/blob/main/LICENSE).
