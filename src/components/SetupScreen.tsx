/**
 * SetupScreen - First-time setup for Voyage DL
 * Allows user to select download folder
 */

import { useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { open } from '@tauri-apps/plugin-dialog';

interface SetupScreenProps {
  onSetupComplete: () => void;
}

export function SetupScreen({ onSetupComplete }: SetupScreenProps) {
  const [loading, setLoading] = useState(false);

  const handleSelectFolder = async () => {
    try {
      setLoading(true);

      // Open folder picker dialog
      const selected = await open({
        directory: true,
        title: 'Sélectionner le dossier de téléchargement',
      });

      if (selected && typeof selected === 'string') {
        // Save the selected directory to config
        await invoke('save_config', {
          config: {
            download_dir: selected,
          },
        });

        // Trigger completion callback
        onSetupComplete();
      }
    } catch (error) {
      console.error('Error selecting folder:', error);
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className="screen setup-screen">
      <div className="setup-screen-content">
        <div className="setup-welcome-icon">
          <svg width="80" height="52" viewBox="0 0 80 52" fill="none" xmlns="http://www.w3.org/2000/svg">
            <rect x="1" y="1" width="78" height="50" rx="6" stroke="currentColor" strokeWidth="2" opacity="0.8"/>
            <rect x="8" y="8" width="26" height="26" rx="13" stroke="currentColor" strokeWidth="2" opacity="0.6"/>
            <circle cx="21" cy="21" r="4" fill="currentColor" opacity="0.5"/>
            <rect x="46" y="8" width="26" height="26" rx="13" stroke="currentColor" strokeWidth="2" opacity="0.6"/>
            <circle cx="59" cy="21" r="4" fill="currentColor" opacity="0.5"/>
            <rect x="28" y="14" width="24" height="14" rx="2" stroke="currentColor" strokeWidth="1.5" opacity="0.3"/>
            <line x1="28" y1="21" x2="52" y2="21" stroke="currentColor" strokeWidth="1" opacity="0.2"/>
            <rect x="12" y="40" width="56" height="5" rx="2.5" fill="currentColor" opacity="0.15"/>
          </svg>
        </div>
        <h1 className="setup-welcome-title">Bienvenue sur Voyage DL</h1>
        <p className="setup-welcome-subtitle">
          Télécharge tes vidéos YouTube et playlists Deezer en MP3.
          <br />
          Commençons par choisir un dossier de destination.
        </p>
        <button
          className={`setup-folder-button ${loading ? 'loading' : ''}`}
          onClick={handleSelectFolder}
          disabled={loading}
        >
          {loading ? (
            <>
              <svg
                className="spinner"
                width="20"
                height="20"
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
              Ouverture du sélecteur...
            </>
          ) : (
            <>
              <span className="icon">📁</span>
              Choisir un dossier de téléchargement
            </>
          )}
        </button>
      </div>
    </div>
  );
}
