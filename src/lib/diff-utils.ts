export function computeDiffStats(original: string, modified: string): { added: number, removed: number } {
    const originalLines = original.split(/\r?\n/);
    const modifiedLines = modified.split(/\r?\n/);
    
    // Simple LCS-based diff to count additions and deletions
    const n = originalLines.length;
    const m = modifiedLines.length;
    
    // Max size to avoid performance issues in browser on huge files
    if (n > 5000 || m > 5000) {
        return { added: 0, removed: 0 }; 
    }

    const dp: number[][] = Array(n + 1).fill(0).map(() => Array(m + 1).fill(0));

    for (let i = 1; i <= n; i++) {
        for (let j = 1; j <= m; j++) {
            // Compare lines content
            if (originalLines[i - 1] === modifiedLines[j - 1]) {
                dp[i][j] = dp[i - 1][j - 1] + 1;
            } else {
                dp[i][j] = Math.max(dp[i - 1][j], dp[i][j - 1]);
            }
        }
    }

    const lcsLen = dp[n][m];
    const removed = n - lcsLen;
    const added = m - lcsLen;

    return { added, removed };
}
