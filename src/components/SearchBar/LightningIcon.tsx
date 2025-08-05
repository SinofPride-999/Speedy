import React from 'react';

const LightningIcon: React.FC<{ className?: string }> = ({ className }) => (
  <svg
    className={className}
    viewBox="0 0 24 24"
    fill="currentColor"
    width="1em"
    height="1em"
  >
    <path d="M7 2v11h3v9l7-12h-4l4-8z" />
  </svg>
);

export default LightningIcon;