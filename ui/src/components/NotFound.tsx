import React from 'react';
import { Link } from 'react-router-dom';
import { Home } from 'lucide-react';

export function NotFound() {
  return (
    <div className="flex flex-col items-center justify-center h-96">
      <h1 className="text-6xl font-bold text-gray-900 mb-4">404</h1>
      <p className="text-xl text-gray-600 mb-8">Page not found</p>
      <Link
        to="/"
        className="flex items-center gap-2 px-4 py-2 bg-primary-600 text-white rounded-lg hover:bg-primary-700 transition-colors"
      >
        <Home className="w-5 h-5" />
        <span>Go to Dashboard</span>
      </Link>
    </div>
  );
}

