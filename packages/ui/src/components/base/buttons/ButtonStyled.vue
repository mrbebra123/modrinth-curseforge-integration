<template>
    <span class="legacy-button-styled">
        <slot />
    </span>
</template>

<style scoped>
.legacy-button-styled {
    display: inline-flex;
    align-items: center;
}

.legacy-button-styled :deep(> button) {
    display: inline-flex;
    min-height: 2.25rem;
    align-items: center;
    justify-content: center;
    gap: 0.375rem;
    padding: 0.375rem 0.625rem;
    border: 0;
    border-radius: 0.75rem;
    background: var(--surface-4);
    color: var(--color-contrast);
    font-weight: 600;
    cursor: pointer;
}

.legacy-button-styled :deep(> button:disabled) {
    opacity: 0.5;
    cursor: not-allowed;
}
</style>
