/**
 * Settings - App configuration modal
 */

import { useState, useEffect } from 'react';
import { open } from '@tauri-apps/plugin-dialog';
import { AppConfig } from '../types';
import { Alert } from './Alert';

interface SettingsProps {
  config: AppConfig;
  onClose: () => void;
  onSave: (config: AppConfig) => Promise<boolean>;
}

export function Settings({ config, onClose, onSave }: SettingsProps) {
  const [formData, setFormData] = useState<AppConfig>(config);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [success, setSuccess] = useState(false);

  useEffect(() => {
    setFormData(config);
  }, [config]);

  const handleChangeFolder = async () => {
    try {
      const selected = await open({
        directory: true,
        title: 'Sélectionner le dossier de téléchargement',
      });

      if (selected && typeof selected === 'string') {
        setFormData((prev) => ({
          ...prev,
          download_dir: selected,
        }));
      }
    } catch (error) {
      console.error('Error selecting folder:', error);
    }
  };

  const handleSave = async () => {
    try {
      setError(null);
      setSuccess(false);
      setLoading(true);

      const success = await onSave(formData);
      if (success) {
        setSuccess(true);
        setTimeout(() => {
          onClose();
        }, 1500);
      }
    } catch (err) {
      const errorMsg = err instanceof Error ? err.message : 'Une erreur est survenue';
      setError(errorMsg);
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className="modal-overlay" onClick={(e) => {
      if (e.target === e.currentTarget) onClose();
    }}>
      <div className="modal">
        <div className="modal-header">
          <h2 className="modal-title">⚙️ Paramètres</h2>
          <button className="modal-close" onClick={onClose}>
            ×
          </button>
        </div>

        <div className="modal-content">
          {error && (
            <Alert
              type="error"
              title="Erreur"
              message={error}
              onClose={() => setError(null)}
            />
          )}

          {success && (
            <Alert
              type="success"
              title="Succès"
              message="Paramètres sauvegardés avec succès"
              onClose={() => setSuccess(false)}
            />
          )}

          {/* Download Folder */}
          <div className="form-group">
            <label className="form-label">📁 Dossier de téléchargement</label>
            <div className="folder-selection">
              <span className="folder-path">
                {formData.download_dir || 'Aucun dossier sélectionné'}
              </span>
              <button
                className="button button-secondary"
                onClick={handleChangeFolder}
                style={{ flex: 'none', padding: 'var(--spacing-sm) var(--spacing-md)' }}
              >
                Parcourir
              </button>
            </div>
          </div>

        </div>

        <div className="modal-footer">
          <button className="button button-secondary" onClick={onClose} disabled={loading}>
            Annuler
          </button>
          <button
            className="button button-primary"
            onClick={handleSave}
            disabled={loading || !formData.download_dir}
          >
            {loading ? (
              <>
                <svg
                  className="spinner"
                  width="16"
                  height="16"
                  viewBox="0 0 24 24"
                  fill="none"
                  stroke="currentColor"
                  strokeWidth="2"
                  strokeLinecap="round"
                  strokeLinejoin="round"
                >
                  <circle cx="12" cy="12" r="1"></circle>
                  <path d="M12 1v6"></path>
                  <path d="M12 17v6"></path>
                  <path d="M4.22 4.22l4.24 4.24"></path>
                  <path d="M15.54 15.54l4.24 4.24"></path>
                  <path d="M1 12h6"></path>
                  <path d="M17 12h6"></path>
                  <path d="M4.22 19.78l4.24-4.24"></path>
                  <path d="M15.54 8.46l4.24-4.24"></path>
                </svg>
                Sauvegarde...
              </>
            ) : (
              <>✓ Sauvegarder</>
            )}
          </button>
        </div>
      </div>
    </div>
  );
}
