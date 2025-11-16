#!/bin/bash
set -e

echo "═══════════════════════════════════════════════════════════════"
echo "  Creating and Merging PR: onion-service-client Feature Fix"
echo "═══════════════════════════════════════════════════════════════"
echo ""

# Create the PR
echo "Step 1: Creating pull request..."
PR_URL=$(gh pr create \
  --base main \
  --title "Fix tor-check: Enable onion-service-client feature for .onion connectivity" \
  --body-file /tmp/pr_body_fix.md)

echo "✅ Pull request created: $PR_URL"
echo ""

# Extract PR number from URL
PR_NUMBER=$(echo "$PR_URL" | grep -oP '\d+$')
echo "PR Number: #$PR_NUMBER"
echo ""

# Merge with confirmation
echo "Step 2: Merging pull request..."
echo "This will merge and delete the branch."
read -p "Continue? (y/n) " -n 1 -r
echo
if [[ $REPLY =~ ^[Yy]$ ]]
then
    gh pr merge "$PR_NUMBER" --squash --delete-branch
    echo ""
    echo "✅ Pull request merged and branch deleted!"
    echo ""
    echo "═══════════════════════════════════════════════════════════════"
    echo "  Success! The tor-check fix is now in main"
    echo "═══════════════════════════════════════════════════════════════"
    echo ""
    echo "What was fixed:"
    echo "  • Added onion-service-client feature to arti-client"
    echo "  • CHECK 3 can now access existing .onion services"
    echo "  • CHECK 5 can now verify round-trip communication"
    echo ""
else
    echo "Merge cancelled."
    echo "You can merge later with: gh pr merge $PR_NUMBER --squash --delete-branch"
fi
