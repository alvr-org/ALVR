function quantile(arr, q) {
    const pos = (arr.length - 1) * q;
    const base = Math.floor(pos);
    const rest = pos - base;
    if (arr[base + 1] !== undefined) {
        return arr[base] + rest * (arr[base + 1] - arr[base]);
    } else {
        return arr[base];
    }
};

function quantiles(arr, min, max) {
    const sorted = arr.sort((a, b) => a - b);
    return [quantile(sorted, min), quantile(sorted, max)];
};
