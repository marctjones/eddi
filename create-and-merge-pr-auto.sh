#!/bin/bash
set -e

echo "═══════════════════════════════════════════════════════════════"
echo "  Creating and Auto-Merging Pull Request"
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

# Auto-merge
echo "Step 2: Auto-merging pull request..."
gh pr merge "$PR_NUMBER" --squash --delete-branch

echo ""
echo "✅ Pull request merged and branch deleted!"
echo ""
echo "═══════════════════════════════════════════════════════════════"
echo "  Success! The tor-check enhancements are now in main"
echo "═══════════════════════════════════════════════════════════════"
echo ""
echo "Summary of merged changes:"
echo "  • CHECK 1: Tor Network Bootstrap"
echo "  • CHECK 2: Access Remote Websites Over Tor"
echo "  • CHECK 3: Access Existing Tor Hidden Services"
echo "  • CHECK 4: Publish Tor Hidden Services"
echo "  • CHECK 5: Verify Round-Trip Communication"
echo ""
