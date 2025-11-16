# Pull Request Scripts

Two scripts have been created to help you create and merge the tor-check enhancement PR:

## Option 1: Interactive Script (Recommended)

```bash
./create-and-merge-pr.sh
```

This script will:
1. Create the pull request
2. Show you the PR details
3. **Ask for confirmation** before merging
4. Merge using squash merge and delete the branch

**Use this if you want to review the PR before merging.**

## Option 2: Auto-Merge Script

```bash
./create-and-merge-pr-auto.sh
```

This script will:
1. Create the pull request
2. **Immediately merge** without asking for confirmation
3. Delete the branch automatically

**Use this if you're confident and want to merge immediately.**

## What Gets Merged

Both scripts merge the comprehensive 5-check tor validation suite:

- ✅ **CHECK 1**: Tor Network Bootstrap
- ✅ **CHECK 2**: Access Remote Websites Over Tor
- ✅ **CHECK 3**: Access Existing Tor Hidden Services
- ✅ **CHECK 4**: Publish Tor Hidden Services
- ✅ **CHECK 5**: Verify Round-Trip Communication

## Manual Commands

If you prefer to run commands manually:

```bash
# Create PR
gh pr create --base main --title "Enhance tor-check with comprehensive 5-check validation suite" --body-file /tmp/pr_body.md

# View PR (replace NUMBER with actual PR number)
gh pr view NUMBER

# Merge PR
gh pr merge NUMBER --squash --delete-branch
```

## Files Involved

- `/tmp/pr_body.md` - PR description (already created)
- `claude/tor-connectivity-checker-01DBCpDcmbTDv1XjKwCu69to` - Branch with all changes
- 4 commits ready to merge

Enjoy your comprehensive Tor connectivity checker! 🧅
