/**
 * Settings - App configuration modal
 */

import { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
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
  const [cacheMessage, setCacheMessage] = useState<string | null>(null);

  useEffect(() => {
    setFormData(config);
  }, [config]);

  const handleChangeFolder = async () => {
    try {
      const selected = await open({
        directory: true,
        title: 'Selectionner le dossier de telechargement',
      });
      if (selected && typeof selected === 'string') {
        setFormData((prev) => ({ ...prev, download_dir: selected }));
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
        setTimeout(() => { onClose(); }, 1500);
      }
    } catch (err) {
      const errorMsg = err instanceof Error ? err.message : 'Une erreur est survenue';
      setError(errorMsg);
    } finally {
      setLoading(false);
    }
  };

  const handleClearYoutubeCache = async () => {
    try {
      const count = await invoke<number>('clear_youtube_cache');
      setCacheMessage(`Cache YouTube vide (${count} entree${count > 1 ? 's' : ''} supprimee${count > 1 ? 's' : ''})`);
      setTimeout(() => setCacheMessage(null), 3000);
    } catch (err) {
      console.error('Failed to clear YouTube cache:', err);
    }
  };

  const handleClearDeezerCache = async () => {
    try {
      const count = await invoke<number>('clear_deezer_cache');
      setCacheMessage(`Cache Deezer vide (${count} entree${count > 1 ? 's' : ''} supprimee${count > 1 ? 's' : ''})`);
      setTimeout(() => setCacheMessage(null), 3000);
    } catch (err) {
      console.error('Failed to clear Deezer cache:', err);
    }
  };

  return (
    <div className="modal-overlay" onClick={(e) => {
      if (e.target === e.currentTarget) onClose();
    }}>
      <div className="modal">
        <div className="modal-header">
          <h2 className="modal-title">Parametres</h2>
          <button className="modal-close" onClick={onClose}>
            x
          </button>
        </div>

        <div className="modal-content">
          {error && (
            <Alert type="error" title="Erreur" message={error} onClose={() => setError(null)} />
          )}
          {success && (
            <Alert type="success" title="Succes" message="Parametres sauvegardes" onClose={() => setSuccess(false)} />
          )}
          {cacheMessage && (
            <Alert type="info" title="Cache" message={cacheMessage} onClose={() => setCacheMessage(null)} />
          )}

          {/* Download Folder */}
          <div className="form-group">
            <label className="form-label">Dossier de telechargement</label>
            <div className="folder-selection">
              <span className="folder-path">
                {formData.download_dir || 'Aucun dossier selectionne'}
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

          {/* Cache management */}
          <div className="form-group">
            <label className="form-label">Gestion du cache</label>
            <p className="form-help-text" style={{ marginBottom: 'var(--spacing-md)' }}>
              Le cache stocke les resultats d'analyse pour accelerer les prochaines recherches.
            </p>
            <div className="cache-actions">
              <button className="cache-clear-btn" onClick={handleClearYoutubeCache}>
                <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
                  <polyline points="3 6 5 6 21 6" />
                  <path d="M19 6v14a2 2 0 01-2 2H7a2 2 0 01-2-2V6m3 0V4a2 2 0 012-2h4a2 2 0 012 2v2" />
                </svg>
                Vider le cache YouTube
              </button>
              <button className="cache-clear-btn" onClick={handleClearDeezerCache}>
                <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
                  <polyline points="3 6 5 6 21 6" />
                  <path d="M19 6v14a2 2 0 01-2 2H7a2 2 0 01-2-2V6m3 0V4a2 2 0 012-2h4a2 2 0 012 2v2" />
                </svg>
                Vider le cache Deezer
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
                <svg className="spinner" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
                  <path d="M12 2v4M12 18v4M4.93 4.93l2.83 2.83M16.24 16.24l2.83 2.83M2 12h4M18 12h4M4.93 19.07l2.83-2.83M16.24 7.76l2.83-2.83" />
                </svg>
                Sauvegarde...
              </>
            ) : (
              'Sauvegarder'
            )}
          </button>
        </div>
      </div>
    </div>
  );
}
