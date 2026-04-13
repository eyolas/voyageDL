/**
 * TrackList - Display and manage selection of tracks
 */

import { useState } from 'react';
import { TrackInfo } from '../types';

interface TrackListProps {
  tracks: TrackInfo[];
  onAddToQueue: (tracks: TrackInfo[]) => void;
}

interface SelectedTrack extends TrackInfo {
  selected: boolean;
}

export function TrackList({ tracks, onAddToQueue }: TrackListProps) {
  const [selectedTracks, setSelectedTracks] = useState<SelectedTrack[]>(
    tracks.map((track) => ({ ...track, selected: true }))
  );

  const selectedCount = selectedTracks.filter((t) => t.selected).length;
  const allSelected = selectedCount === selectedTracks.length;

  const handleToggleAll = () => {
    setSelectedTracks((prev) =>
      prev.map((track) => ({
        ...track,
        selected: !allSelected,
      }))
    );
  };

  const handleToggleTrack = (trackId: string) => {
    setSelectedTracks((prev) =>
      prev.map((track) =>
        track.id === trackId ? { ...track, selected: !track.selected } : track
      )
    );
  };

  const formatDuration = (seconds: number): string => {
    const mins = Math.floor(seconds / 60);
    const secs = seconds % 60;
    return `${mins}:${secs.toString().padStart(2, '0')}`;
  };

  const handleDownload = () => {
    if (selectedCount === 0) return;
    const tracksToDownload: TrackInfo[] = selectedTracks
      .filter((t) => t.selected)
      .map(({ selected, ...track }) => track);
    onAddToQueue(tracksToDownload);
  };

  return (
    <div className="track-list">
      <div className="track-list-header">
        <h3 className="track-list-title">Pistes disponibles</h3>
        <button className="toggle-select" onClick={handleToggleAll}>
          {allSelected ? 'Tout deselectionner' : 'Tout selectionner'}
        </button>
      </div>

      <div className="track-list-items">
        {selectedTracks.map((track) => (
          <label
            key={track.id}
            className={`track-item ${track.selected ? 'selected' : ''}`}
          >
            <input
              type="checkbox"
              className="track-checkbox"
              checked={track.selected}
              onChange={() => handleToggleTrack(track.id)}
            />
            <div className="track-info">
              <div className="track-title">{track.title}</div>
              <div className="track-meta">
                <span className="track-artist">{track.artist}</span>
                <span className="track-duration">{formatDuration(track.duration_seconds)}</span>
              </div>
            </div>
          </label>
        ))}
      </div>

      <div className="track-list-footer">
        <span className="track-count">
          {selectedCount} sur {selectedTracks.length} selectionnee{selectedCount !== 1 ? 's' : ''}
        </span>
        <button
          className="download-button"
          onClick={handleDownload}
          disabled={selectedCount === 0}
        >
          <span>⬇️</span>
          Telecharger ({selectedCount})
        </button>
      </div>
    </div>
  );
}
