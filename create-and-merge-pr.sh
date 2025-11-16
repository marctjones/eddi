#!/bin/bash
set -e

echo "═══════════════════════════════════════════════════════════════"
echo "  Creating Pull Request for Tor Check Enhancements"
echo "═══════════════════════════════════════════════════════════════"
echo ""

# Create the PR
echo "Step 1: Creating pull request..."
PR_URL=$(gh pr create \
  --base main \
  --title "Enhance tor-check with comprehensive 5-check validation suite" \
  --body-file /tmp/pr_body.md)

echo "✅ Pull request created: $PR_URL"
echo ""

# Extract PR number from URL
PR_NUMBER=$(echo "$PR_URL" | grep -oP '\d+$')
echo "PR Number: #$PR_NUMBER"
echo ""

# Show PR details
echo "Step 2: Showing PR details..."
gh pr view "$PR_NUMBER"
echo ""

# Ask for confirmation before merging
read -p "Do you want to merge this PR? (y/N) " -n 1 -r
echo ""

if [[ $REPLY =~ ^[Yy]$ ]]
then
    echo "Step 3: Merging pull request..."
    gh pr merge "$PR_NUMBER" --squash --delete-branch
    echo ""
    echo "✅ Pull request merged and branch deleted!"
    echo ""
    echo "═══════════════════════════════════════════════════════════════"
    echo "  Success! The tor-check enhancements are now in main"
    echo "═══════════════════════════════════════════════════════════════"
else
    echo ""
    echo "Merge cancelled. You can merge later with:"
    echo "  gh pr merge $PR_NUMBER --squash --delete-branch"
fi
