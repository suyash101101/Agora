import React, { useState } from 'react';
import { generateSalt, generateCommitHash, generateCommitHashBytes, hashHexToHash } from '../../utils/helpers';
import { stringToBytes, bytesToString } from '../../utils/formatters';
import { Copy, Check, RefreshCw } from 'lucide-react';

/**
 * Hash Generator Component
 * Allows users to generate commit hashes for commit-reveal scheme
 * Inspired by hash_demo logic for easy hash generation
 */
export function HashGenerator() {
  const [result, setResult] = useState('');
  const [salt, setSalt] = useState<Uint8Array | null>(null);
  const [commitHash, setCommitHash] = useState<string | null>(null);
  const [saltAscii, setSaltAscii] = useState<string>('');
  const [copied, setCopied] = useState<'salt' | 'hash' | null>(null);

  const handleGenerateSalt = () => {
    // Generate random salt with printable ASCII characters (a-z, A-Z, 0-9, special chars)
    const chars = 'abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789!@#$%^&*()-_=+[]{}|;:,.<>?';
    let saltString = '';
    const randomBytes = new Uint8Array(32);
    crypto.getRandomValues(randomBytes);
    
    for (let i = 0; i < 32; i++) {
      saltString += chars[randomBytes[i] % chars.length];
    }
    
    const newSalt = stringToBytes(saltString);
    setSalt(newSalt);
    setSaltAscii(saltString);
    setCommitHash(null); // Reset hash when salt changes
  };

  const handleGenerateHash = () => {
    if (!result.trim()) {
      alert('Please enter a result first');
      return;
    }
    if (!salt) {
      alert('Please generate a salt first');
      return;
    }

    const resultBytes = stringToBytes(result);
    const hash = generateCommitHash(salt, resultBytes);
    setCommitHash(hash);
  };

  const handleCopy = async (type: 'salt' | 'hash', value: string) => {
    try {
      await navigator.clipboard.writeText(value);
      setCopied(type);
      setTimeout(() => setCopied(null), 2000);
    } catch (error) {
      console.error('Failed to copy:', error);
    }
  };

  const handleSaltFromAscii = (ascii: string) => {
    try {
      if (ascii.length !== 32) {
        alert('Salt must be exactly 32 ASCII characters (32 bytes)');
        return;
      }

      const saltBytes = stringToBytes(ascii);
      setSalt(saltBytes);
      setSaltAscii(ascii);
      setCommitHash(null);
    } catch (error) {
      alert('Invalid ASCII format. Must be 32 ASCII characters.');
    }
  };

  return (
    <div className="bg-white rounded-lg border border-gray-200 p-6">
      <h2 className="text-xl font-semibold mb-4">Hash Generator</h2>
      <p className="text-sm text-gray-600 mb-6">
        Generate commit hashes for commit-reveal scheme. Enter your result and generate a salt to create a commit hash.
      </p>

      <div className="space-y-4">
        {/* Result Input */}
        <div>
          <label className="block text-sm font-medium text-gray-700 mb-2">
            Result (to hash)
          </label>
          <textarea
            value={result}
            onChange={(e) => {
              setResult(e.target.value);
              setCommitHash(null); // Reset hash when result changes
            }}
            placeholder="Enter your result..."
            rows={4}
            className="w-full px-4 py-2 border border-gray-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-primary-500"
          />
        </div>

        {/* Salt Generation */}
        <div>
          <label className="block text-sm font-medium text-gray-700 mb-2">
            Salt (32 bytes ASCII)
          </label>
          <div className="flex gap-2">
            <button
              onClick={handleGenerateSalt}
              className="px-4 py-2 bg-gray-200 text-gray-700 rounded-lg hover:bg-gray-300 transition-colors flex items-center gap-2"
            >
              <RefreshCw className="w-4 h-4" />
              Generate Random Salt
            </button>
            <input
              type="text"
              value={saltAscii}
              onChange={(e) => {
                setSaltAscii(e.target.value);
                if (e.target.value.length === 32) {
                  handleSaltFromAscii(e.target.value);
                }
              }}
              placeholder="Or enter 32 ASCII characters"
              maxLength={32}
              className="flex-1 px-4 py-2 border border-gray-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-primary-500 font-mono text-sm"
            />
          </div>
          {salt && (
            <div className="mt-2 bg-gray-50 p-3 rounded-lg">
              <div className="flex items-center justify-between">
                <div className="flex-1">
                  <p className="text-xs text-gray-500 mb-1">Salt (ASCII, {saltAscii.length} chars)</p>
                  <p className="font-mono text-sm break-all">
                    {saltAscii}
                  </p>
                </div>
                <button
                  onClick={() => handleCopy('salt', saltAscii)}
                  className="ml-2 p-2 text-gray-500 hover:text-gray-700 transition-colors"
                  title="Copy salt"
                >
                  {copied === 'salt' ? (
                    <Check className="w-5 h-5 text-green-600" />
                  ) : (
                    <Copy className="w-5 h-5" />
                  )}
                </button>
              </div>
            </div>
          )}
        </div>

        {/* Generate Hash Button */}
        <button
          onClick={handleGenerateHash}
          disabled={!result.trim() || !salt}
          className="w-full px-4 py-2 bg-primary-600 text-white rounded-lg hover:bg-primary-700 transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
        >
          Generate Commit Hash
        </button>

        {/* Commit Hash Display */}
        {commitHash && (
          <div className="bg-gray-50 p-4 rounded-lg">
            <div className="flex items-center justify-between mb-2">
              <p className="text-xs text-gray-500">Commit Hash</p>
              <button
                onClick={() => handleCopy('hash', commitHash)}
                className="p-1 text-gray-500 hover:text-gray-700 transition-colors"
                title="Copy hash"
              >
                {copied === 'hash' ? (
                  <Check className="w-5 h-5 text-green-600" />
                ) : (
                  <Copy className="w-5 h-5" />
                )}
              </button>
            </div>
            <p className="font-mono text-sm break-all">{commitHash}</p>
            <p className="text-xs text-gray-500 mt-2">
              This hash is calculated as: blake2_256(salt + result)
            </p>
          </div>
        )}

        {/* Instructions */}
        <div className="bg-blue-50 border border-blue-200 p-4 rounded-lg">
          <p className="text-sm font-medium text-blue-900 mb-2">How to use:</p>
          <ol className="text-xs text-blue-800 space-y-1 list-decimal list-inside">
            <li>Enter your result in the text area</li>
            <li>Click "Generate Random Salt" or enter a 32-character ASCII salt</li>
            <li>Click "Generate Commit Hash" to create the commit hash</li>
            <li>Copy the salt and hash values</li>
            <li>Use these values when committing results in the Commit/Reveal interface</li>
          </ol>
        </div>
      </div>
    </div>
  );
}