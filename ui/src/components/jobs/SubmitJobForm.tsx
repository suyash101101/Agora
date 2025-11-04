import React, { useState } from 'react';
import { useApiContext } from '../../context/ApiContext';
import { useAccountContext } from '../../context/AccountContext';
import { MIN_JOB_BOUNTY, JobType, JOB_TYPE_LABELS } from '../../utils/constants';
import { parseBalance } from '../../utils/formatters';
import { validateJobInput, validateBounty } from '../../utils/helpers';
import { stringToBytes } from '../../utils/formatters';
import { Send, Loader } from 'lucide-react';

export function SubmitJobForm() {
  const { api } = useApiContext();
  const { account, getSigner } = useAccountContext();
  const [jobType, setJobType] = useState<JobType>(JobType.ApiRequest);
  const [inputData, setInputData] = useState('');
  const [bounty, setBounty] = useState('');
  const [isSubmitting, setIsSubmitting] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!api || !account) {
      setError('Please connect your wallet first');
      return;
    }

    setError(null);

    // Validate input
    const inputValidation = validateJobInput(inputData);
    if (!inputValidation.valid) {
      setError(inputValidation.error || 'Invalid input');
      return;
    }

    const bountyBN = parseBalance(bounty);
    const bountyValidation = validateBounty(BigInt(bountyBN.toString()), MIN_JOB_BOUNTY);
    if (!bountyValidation.valid) {
      setError(bountyValidation.error || 'Invalid bounty');
      return;
    }

    try {
      setIsSubmitting(true);
      const signer = await getSigner(account.address);
      const inputBytes = stringToBytes(inputData);
      
      const tx = api.tx.agora.submitJob(jobType, inputBytes, bountyBN.toString());
      
      await tx.signAndSend(account.address, { signer }, ({ status }) => {
        if (status.isInBlock) {
          console.log('Submitted in block:', status.asInBlock.toString());
          setInputData('');
          setBounty('');
          setIsSubmitting(false);
          alert('Job submitted successfully!');
        } else if (status.isFinalized) {
          setIsSubmitting(false);
        }
      });
    } catch (error) {
      console.error('Failed to submit job:', error);
      setError(error instanceof Error ? error.message : 'Failed to submit job');
      setIsSubmitting(false);
    }
  };

  if (!account) {
    return (
      <div className="bg-white rounded-lg border border-gray-200 p-6">
        <p className="text-gray-500">Please connect your wallet to submit a job</p>
      </div>
    );
  }

  return (
    <div className="bg-white rounded-lg border border-gray-200 p-6">
      <h2 className="text-xl font-semibold mb-4">Submit New Job</h2>
      <form onSubmit={handleSubmit} className="space-y-4">
        <div>
          <label className="block text-sm font-medium text-gray-700 mb-2">
            Job Type
          </label>
          <select
            value={jobType}
            onChange={(e) => setJobType(parseInt(e.target.value) as JobType)}
            className="w-full px-4 py-2 border border-gray-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-primary-500"
          >
            {Object.entries(JOB_TYPE_LABELS).map(([value, label]) => (
              <option key={value} value={value}>
                {label}
              </option>
            ))}
          </select>
        </div>

        <div>
          <label className="block text-sm font-medium text-gray-700 mb-2">
            Input Data
          </label>
          <textarea
            value={inputData}
            onChange={(e) => setInputData(e.target.value)}
            placeholder="Enter job input data..."
            rows={4}
            className="w-full px-4 py-2 border border-gray-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-primary-500"
          />
          <p className="text-xs text-gray-500 mt-1">
            Maximum 1024 bytes
          </p>
        </div>

        <div>
          <label className="block text-sm font-medium text-gray-700 mb-2">
            Bounty (UNIT)
          </label>
          <input
            type="text"
            value={bounty}
            onChange={(e) => setBounty(e.target.value)}
            placeholder={`Minimum: ${MIN_JOB_BOUNTY.toString()}`}
            className="w-full px-4 py-2 border border-gray-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-primary-500"
          />
        </div>

        {error && (
          <div className="bg-red-50 border border-red-200 text-red-700 px-4 py-3 rounded-lg">
            {error}
          </div>
        )}

        <button
          type="submit"
          disabled={isSubmitting || !inputData.trim() || !bounty.trim()}
          className="w-full flex items-center justify-center gap-2 px-4 py-2 bg-primary-600 text-white rounded-lg hover:bg-primary-700 transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
        >
          {isSubmitting ? (
            <>
              <Loader className="w-5 h-5 animate-spin" />
              <span>Submitting...</span>
            </>
          ) : (
            <>
              <Send className="w-5 h-5" />
              <span>Submit Job</span>
            </>
          )}
        </button>
      </form>
    </div>
  );
}

