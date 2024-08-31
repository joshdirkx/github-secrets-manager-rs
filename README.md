# GitHub Secrets Manager

A Rust application for managing GitHub repository secrets programmatically 
using the GitHub API. This application fetches the public key for a 
repository, encrypts secrets using the `sodiumoxide` crate, and creates, 
updates, or deletes secrets in the GitHub repository based on the provided 
JSON array of key/value pairs.

## Features

- **Create Secrets**: Creates new secrets in the GitHub repository from a 
  provided JSON array.
- **Update Secrets**: Updates existing secrets in the GitHub repository from 
  a provided JSON array.
- **Delete Secrets**: Identifies and deletes secrets that are no longer 
  included in the JSON array.

## Prerequisites

- **Rust Installed**: Ensure you have Rust installed on your machine. Follow 
  the [official Rust installation guide](https://www.rust-lang.org/tools/install) for instructions.
- **GitHub Personal Access Token**: You'll need a GitHub personal access 
  token with the appropriate permissions to update secrets in the repository.

## Setup

### Generating a GitHub Personal Access Token

To use this application, you'll need to generate a personal access token 
(PAT) on GitHub with the required permissions. Follow these steps:

1. **Sign in to GitHub**: Go to [github.com](https://github.com) and log in 
   to your account.

2. **Navigate to Personal Access Tokens**:
   - Click on your profile picture in the top-right corner and select `Settings`.
   - In the left sidebar, click on `Developer settings`.
   - Under `Personal access tokens`, click on `Tokens (classic)`.

3. **Generate a New Token**:
   - Click `Generate new token`.
   - Provide a descriptive name for the token in the `Note` field.
   - Under `Select scopes`, choose the following permissions:
     - `repo`
     - `workflow`
     - `admin:org`
   - You can choose additional scopes if needed.

4. **Generate and Copy the Token**:
   - Scroll down and click `Generate token`.
   - **Important**: Copy the token immediately. You will not be able to see 
     it again once you leave this page.

5. **Store the Token Securely**:
   - Save the token in a secure place, as you'll need to set it in the 
     `GITHUB_TOKEN` environment variable when running this application.

### Setting up Environment Variables for Your Secrets

Before running the application, you need to set up the following environment 
variables in your `.env` file (recommended) or as system environment variables:

- `GITHUB_ORGANIZATION`: The GitHub username or organization name that owns 
  the repository.
- `GITHUB_REPOSITORY`: The name of the repository where you want to manage 
  secrets.
- `GITHUB_TOKEN`: Your GitHub personal access token.
- `GITHUB_SECRETS`: A JSON array containing the secrets you want to manage. 
  Each secret is a key/value pair.

#### Using a `.env` File

Copy `.env.example` to `.env` and update it with your values accordingly.

#### Using System Environment Variables

```bash
export GITHUB_ORGANIZATION="organization"
export GITHUB_REPOSITORY="repository"
export GITHUB_TOKEN="token"
export GITHUB_SECRETS='[
    {"name": "SECRET_NAME_1", "value": "secret_value_1"},
    {"name": "SECRET_NAME_2", "value": "secret_value_2"}
]'
```

## Usage

To run the application, execute the following command:

```bash
cargo run
```

### Output

After running the script, you'll see output messages indicating:
- Which secrets are new and have been created.
- Which existing secrets have been updated.
- Which secrets have been deleted.

This ensures that your GitHub repository's secrets are always in sync with your specified configuration.