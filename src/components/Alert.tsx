/**
 * Alert component for displaying messages
 */

interface AlertProps {
  type: 'info' | 'success' | 'error' | 'warning';
  title: string;
  message: string;
  onClose?: () => void;
}

export function Alert({ type, title, message, onClose }: AlertProps) {
  const icons: Record<string, string> = {
    info: 'ℹ️',
    success: '✓',
    error: '✕',
    warning: '⚠️',
  };

  return (
    <div className={`alert ${type}`}>
      <span className="alert-icon">{icons[type]}</span>
      <div className="alert-content">
        <div className="alert-title">{title}</div>
        <div className="alert-message">{message}</div>
      </div>
      {onClose && (
        <button className="alert-close" onClick={onClose}>
          ×
        </button>
      )}
    </div>
  );
}
